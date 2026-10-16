use crate::{library::Track, Store};
use rusqlite::{params, Connection};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashSet;

#[derive(Serialize)]
pub struct CommunityPlaylist {
    id: String,
    title: String,
    author: String,
    thumbnail: String,
}

fn playlist_id(id: &str) -> Result<&str, String> {
    if !(id.starts_with("PL") && (12..=128).contains(&id.len())
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')) {
        return Err("Invalid public community playlist ID".into());
    }
    Ok(id)
}

fn playlist_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 120 || name.chars().any(char::is_control) {
        return Err("Playlist name must contain 1–120 characters without control characters".into());
    }
    Ok(name)
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().timeout(std::time::Duration::from_secs(20))
        .user_agent("Mozilla/5.0").build().map_err(|e| e.to_string())
}

async fn request(client: &reqwest::Client, endpoint: &str, mut body: Value) -> Result<Value, String> {
    body["context"] = json!({"client":{"clientName":"WEB_REMIX","clientVersion":"1.20261012.01.00","hl":"en","gl":"ID"}});
    let value: Value = client.post(format!("https://music.youtube.com/youtubei/v1/{endpoint}?prettyPrint=false"))
        .json(&body).send().await.map_err(|e| format!("YouTube Music request failed; check your connection and retry: {e}"))?
        .error_for_status().map_err(|e| format!("YouTube Music rejected the request: {e}"))?
        .json().await.map_err(|e| format!("Invalid YouTube Music response: {e}"))?;
    if value.get("error").is_some() {
        return Err("YouTube Music could not load this content; it may be private or unavailable".into());
    }
    Ok(value)
}

fn column(row: &Value, index: usize) -> Option<&str> {
    row.get("flexColumns")?.get(index)?.pointer("/musicResponsiveListItemFlexColumnRenderer/text/runs/0/text")?.as_str()
}

fn parse_playlist(row: &Value) -> Option<CommunityPlaylist> {
    let endpoint = row.pointer("/navigationEndpoint/browseEndpoint")?;
    if endpoint.pointer("/browseEndpointContextSupportedConfigs/browseEndpointContextMusicConfig/pageType")?.as_str()?
        != "MUSIC_PAGE_TYPE_PLAYLIST" { return None; }
    let id = endpoint.get("browseId")?.as_str()?.strip_prefix("VL")?;
    playlist_id(id).ok()?;
    let title = column(row, 0)?.trim();
    if title.is_empty() || title.len() > 4096 { return None; }
    let author = column(row, 1).unwrap_or("");
    if author.len() > 4096 { return None; }
    let thumbnail = row.pointer("/thumbnail/musicThumbnailRenderer/thumbnail/thumbnails")
        .and_then(Value::as_array).and_then(|images| images.last())
        .and_then(|image| image.get("url")).and_then(Value::as_str)
        .filter(|url| url.starts_with("https://") && url.len() <= 4096).unwrap_or("");
    Some(CommunityPlaylist { id:id.into(), title:title.into(), author:author.into(), thumbnail:thumbnail.into() })
}

#[tauri::command]
pub async fn community_playlists(query: String) -> Result<Vec<CommunityPlaylist>, String> {
    let query = query.trim();
    if query.chars().count() > 200 || query.chars().any(char::is_control) {
        return Err("Search must contain at most 200 characters without control characters".into());
    }
    // The community-only filter excludes YouTube's curated playlists, albums and videos.
    let value = request(&client()?, "search", json!({"query":if query.is_empty() { "chill music" } else { query },
        "params":"EgeKAQQoAEABagwQAxAEEAkQChAFEBA%3D"})).await?;
    let sections = value.pointer("/contents/tabbedSearchResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents")
        .and_then(Value::as_array).ok_or("YouTube Music search format changed; retry or update Sela")?;
    let mut playlists = Vec::new();
    for section in sections {
        if let Some(items) = section.pointer("/musicShelfRenderer/contents").and_then(Value::as_array) {
            for item in items {
                if let Some(playlist) = item.get("musicResponsiveListItemRenderer").and_then(parse_playlist) {
                    if !playlists.iter().any(|p: &CommunityPlaylist| p.id == playlist.id) { playlists.push(playlist); }
                }
            }
        }
    }
    Ok(playlists)
}

fn track_page(value: &Value, continuation: bool) -> Result<(Vec<Track>, Option<String>), String> {
    let shelf = if continuation {
        value.pointer("/continuationContents/musicPlaylistShelfContinuation")
            .or_else(|| value.pointer("/continuationContents/musicShelfContinuation"))
    } else {
        let sections = value.pointer("/contents/twoColumnBrowseResultsRenderer/secondaryContents/sectionListRenderer/contents")
            .or_else(|| value.pointer("/contents/singleColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents"))
            .and_then(Value::as_array);
        sections.and_then(|sections| sections.iter().find_map(|section| section.get("musicPlaylistShelfRenderer")))
    };
    let items = shelf.and_then(|s| s.get("contents")).and_then(Value::as_array)
        .or_else(|| if continuation {
            value.get("onResponseReceivedActions").and_then(Value::as_array)
                .and_then(|actions| actions.iter().find_map(|a| a.pointer("/appendContinuationItemsAction/continuationItems").and_then(Value::as_array)))
        } else { None })
        .ok_or("Cannot read the complete playlist; it may be private, removed, or YouTube Music changed its format. Nothing was saved")?;
    let mut token = shelf.and_then(|s| s.pointer("/continuations/0/nextContinuationData/continuation"))
        .and_then(Value::as_str).map(str::to_owned);
    if shelf.is_some_and(|s| s.get("continuations").and_then(Value::as_array).is_some_and(|c| !c.is_empty())) && token.is_none() {
        return Err("Unsupported playlist pagination; update Sela before importing. Nothing was saved".into());
    }
    let mut tracks = Vec::new();
    for item in items {
        if let Some(next) = item.get("continuationItemRenderer") {
            let next = next.pointer("/continuationEndpoint/continuationCommand/token").and_then(Value::as_str)
                .filter(|s| !s.is_empty()).ok_or("Missing playlist continuation token; nothing was saved")?;
            token = Some(next.into());
            continue;
        }
        let Some(row) = item.get("musicResponsiveListItemRenderer") else {
            return Err("Unexpected playlist entry format; update Sela before importing. Nothing was saved".into());
        };
        if row.get("musicItemRendererDisplayPolicy").and_then(Value::as_str) == Some("MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT") { continue; }
        let Some(id) = row.pointer("/playlistItemData/videoId").and_then(Value::as_str) else { continue; };
        let Some(title) = column(row, 0) else { return Err("Playlist song title is missing; nothing was saved".into()); };
        let channel = column(row, 1);
        if id.len() != 11 || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            || title.trim().is_empty() || title.len() > 4096 || channel.is_some_and(|s| s.len() > 4096) {
            return Err("Playlist contains invalid song metadata; nothing was saved".into());
        }
        tracks.push(Track { id:id.into(), title:title.into(), channel:channel.map(str::to_owned) });
    }
    Ok((tracks, token))
}

#[tauri::command]
pub async fn community_playlist_tracks(id: String) -> Result<Vec<Track>, String> {
    playlist_id(&id)?;
    let client = client()?;
    let value = request(&client, "browse", json!({"browseId":format!("VL{id}")})).await?;
    let (mut tracks, mut token) = track_page(&value, false)?;
    let mut seen = HashSet::new();
    while let Some(next) = token {
        // Bound a broken upstream response, never return or save a truncated playlist.
        if seen.len() >= 1000 || !seen.insert(next.clone()) {
            return Err("YouTube Music pagination did not finish; retry this playlist later. Nothing was saved".into());
        }
        let value = request(&client, "browse", json!({"continuation":next})).await?;
        let (page, next_token) = track_page(&value, true)?;
        tracks.extend(page);
        token = next_token;
    }
    if tracks.is_empty() { return Err("This public playlist has no available songs".into()); }
    Ok(tracks)
}

fn persist(db: &mut Connection, name: &str, tracks: &[Track]) -> Result<i64, String> {
    let transaction = db.transaction().map_err(|e| e.to_string())?;
    transaction.execute("INSERT INTO playlists(name) VALUES (?1)", [name]).map_err(|e| e.to_string())?;
    let id = transaction.last_insert_rowid();
    {
        let mut insert = transaction.prepare("INSERT INTO playlist_entries(playlist_id,video_id,title,channel,position) VALUES (?1,?2,?3,?4,?5)").map_err(|e| e.to_string())?;
        for (position, track) in tracks.iter().enumerate() {
            insert.execute(params![id, track.id, track.title, track.channel, position as i64]).map_err(|e| e.to_string())?;
        }
    }
    transaction.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub async fn save_community_playlist(id: String, name: String, store: tauri::State<'_, Store>) -> Result<i64, String> {
    let name = playlist_name(&name)?;
    let tracks = community_playlist_tracks(id).await?;
    let mut db = store.0.lock().map_err(|e| e.to_string())?;
    persist(&mut db, name, &tracks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song(id: &str) -> Value {
        json!({"musicResponsiveListItemRenderer": {"playlistItemData":{"videoId":id},
            "flexColumns":[{"musicResponsiveListItemFlexColumnRenderer":{"text":{"runs":[{"text":"Song"}]}}}]}})
    }

    #[test]
    fn validates_external_identifiers_and_names() {
        assert!(playlist_id("PLd9TE8N_C0s0").is_ok());
        for id in ["", "VLPLabc", "MPREalbum", "PL../secret", "PLa?x=1"] {
            assert!(playlist_id(id).is_err());
        }
        assert!(playlist_name("  ").is_err());
        assert!(playlist_name(&"a".repeat(121)).is_err());
        assert_eq!(playlist_name(" Test ").unwrap(), "Test");
    }

    #[test]
    fn search_accepts_playlist_page_type_only() {
        let row = json!({"navigationEndpoint":{"browseEndpoint":{"browseId":"VLPLd9TE8N_C0s0",
            "browseEndpointContextSupportedConfigs":{"browseEndpointContextMusicConfig":{"pageType":"MUSIC_PAGE_TYPE_PLAYLIST"}}}},
            "flexColumns":[{"musicResponsiveListItemFlexColumnRenderer":{"text":{"runs":[{"text":"Real playlist"}]}}}]});
        assert_eq!(parse_playlist(&row).unwrap().id, "PLd9TE8N_C0s0");
        let mut album = row.clone();
        album["navigationEndpoint"]["browseEndpoint"]["browseEndpointContextSupportedConfigs"]["browseEndpointContextMusicConfig"]["pageType"] = json!("MUSIC_PAGE_TYPE_ALBUM");
        assert!(parse_playlist(&album).is_none());
    }

    #[test]
    fn tracks_preserve_duplicates_skip_unavailable_and_follow_only_track_tokens() {
        let mut unavailable = song("abcdefghijk");
        unavailable["musicResponsiveListItemRenderer"]["musicItemRendererDisplayPolicy"] = json!("MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT");
        let value = json!({"contents":{"twoColumnBrowseResultsRenderer":{"secondaryContents":{"sectionListRenderer":{
            "contents":[{"musicPlaylistShelfRenderer":{"contents":[song("dQw4w9WgXcQ"), unavailable, song("dQw4w9WgXcQ"),
                {"continuationItemRenderer":{"continuationEndpoint":{"continuationCommand":{"token":"tracks-next"}}}}]}}],
            "continuations":[{"nextContinuationData":{"continuation":"related-playlists"}}]
        }}}}});
        let (tracks, token) = track_page(&value, false).unwrap();
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].id, tracks[1].id);
        assert_eq!(token.as_deref(), Some("tracks-next"));
        let next = json!({"onResponseReceivedActions":[{"appendContinuationItemsAction":{"continuationItems":[song("abcdefghijk")]}}]});
        let (tracks, token) = track_page(&next, true).unwrap();
        assert_eq!(tracks[0].id, "abcdefghijk");
        assert!(token.is_none());
        assert!(track_page(&json!({"error":{}}), true).is_err());
    }

    #[test]
    fn saving_is_ordered_and_rolls_back_entire_playlist_on_failure() {
        let mut db = rusqlite::Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE playlists(id INTEGER PRIMARY KEY,name TEXT NOT NULL);").unwrap();
        crate::library::initialize(&db).unwrap();
        let track = |id: &str| Track { id:id.into(), title:"Song".into(), channel:None };
        let id = persist(&mut db, "Saved", &[track("dQw4w9WgXcQ"),track("abcdefghijk"),track("dQw4w9WgXcQ")]).unwrap();
        let saved = crate::library::entries(&db,id).unwrap();
        assert_eq!(saved.len(),3);
        assert_eq!(saved[1].track.id,"abcdefghijk");
        db.execute_batch("CREATE TRIGGER reject_entry BEFORE INSERT ON playlist_entries WHEN NEW.video_id='abcdefghijk' BEGIN SELECT RAISE(ABORT,'disk failure'); END;").unwrap();
        assert!(persist(&mut db,"Failed", &[track("dQw4w9WgXcQ"),track("abcdefghijk")]).is_err());
        assert_eq!(db.query_row("SELECT count(*) FROM playlists",[],|r|r.get::<_,i64>(0)).unwrap(),1);
        assert_eq!(db.query_row("SELECT count(*) FROM playlist_entries",[],|r|r.get::<_,i64>(0)).unwrap(),3);
    }
}
