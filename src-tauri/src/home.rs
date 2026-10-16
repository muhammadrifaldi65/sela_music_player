use crate::library::Track;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Serialize)]
pub struct HomeShelf {
    pub id: &'static str,
    pub title: &'static str,
    pub items: Vec<Track>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn source_shelf<'a>(value: &'a Value, category: &str) -> Option<&'a Vec<Value>> {
    let sections = value.pointer("/contents/singleColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents")?.as_array()?;
    sections.iter().find_map(|section| {
        let shelf = section.get("musicCarouselShelfRenderer")?;
        let header = shelf.pointer("/header/musicCarouselShelfBasicHeaderRenderer/title/runs/0")?;
        let matches = match category {
            "new-releases" => {
                header
                    .pointer("/navigationEndpoint/browseEndpoint/browseId")
                    .and_then(Value::as_str)
                    == Some("FEmusic_new_releases_albums")
            }
            "trending" => header.get("text").and_then(Value::as_str) == Some("Trending"),
            _ => false,
        };
        matches.then(|| shelf.get("contents")?.as_array()).flatten()
    })
}

fn append_tracks(items: &[Value], tracks: &mut Vec<Track>) {
    for item in items {
        if tracks.len() >= 24 {
            break;
        }
        let fields = if let Some(row) = item.get("musicResponsiveListItemRenderer") {
            if row
                .get("musicItemRendererDisplayPolicy")
                .and_then(Value::as_str)
                == Some("MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT")
            {
                continue;
            }
            (
                row.pointer("/playlistItemData/videoId"),
                row.pointer(
                    "/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/text",
                ),
                row.pointer(
                    "/flexColumns/1/musicResponsiveListItemFlexColumnRenderer/text/runs/0/text",
                ),
            )
        } else if let Some(row) = item.get("musicTwoRowItemRenderer") {
            (
                row.pointer("/navigationEndpoint/watchEndpoint/videoId"),
                row.pointer("/title/runs/0/text"),
                row.pointer("/subtitle/runs/0/text"),
            )
        } else {
            continue;
        };
        if let (Some(id), Some(title)) = (
            fields.0.and_then(Value::as_str),
            fields.1.and_then(Value::as_str),
        ) {
            if !id.is_empty() && !title.is_empty() && !tracks.iter().any(|track| track.id == id) {
                tracks.push(Track {
                    id: id.into(),
                    title: title.into(),
                    channel: fields.2.and_then(Value::as_str).map(str::to_owned),
                });
            }
        }
    }
}

fn append_album_tracks(value: &Value, tracks: &mut Vec<Track>) {
    // Only the album's track list; never descend into related-release carousels.
    if let Some(sections) = value.pointer("/contents/twoColumnBrowseResultsRenderer/secondaryContents/sectionListRenderer/contents").and_then(Value::as_array) {
        for section in sections {
            if let Some(items) = section.pointer("/musicShelfRenderer/contents").and_then(Value::as_array) {
                append_tracks(items, tracks);
                break;
            }
        }
    }
}

async fn browse(client: &reqwest::Client, id: &str) -> Result<Value, String> {
    client.post("https://music.youtube.com/youtubei/v1/browse?prettyPrint=false")
        .json(&json!({"context":{"client":{"clientName":"WEB_REMIX","clientVersion":"1.20261012.01.00","hl":"en","gl":"ID"}},"browseId":id}))
        .send().await.map_err(|e| format!("YouTube Music request failed: {e}"))?
        .error_for_status().map_err(|e| format!("YouTube Music rejected request: {e}"))?
        .json().await.map_err(|e| format!("Invalid YouTube Music response: {e}"))
}

#[tauri::command]
pub async fn music_home() -> Result<Vec<HomeShelf>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .user_agent("Mozilla/5.0")
        .build()
        .map_err(|e| e.to_string())?;
    let mut shelves = vec![
        HomeShelf {
            id: "new-releases",
            title: "New releases",
            items: Vec::new(),
            error: None,
        },
        HomeShelf {
            id: "trending",
            title: "Trending",
            items: Vec::new(),
            error: None,
        },
    ];
    let explore = match browse(&client, "FEmusic_explore").await {
        Ok(value) => value,
        Err(error) => {
            for shelf in &mut shelves {
                shelf.error = Some(error.clone());
            }
            return Ok(shelves);
        }
    };
    for shelf in &mut shelves {
        let Some(items) = source_shelf(&explore, shelf.id) else {
            shelf.error = Some("YouTube Music did not provide this category".into());
            continue;
        };
        append_tracks(items, &mut shelf.items);
        if shelf.id == "new-releases" {
            // ponytail: first three source albums, no continuations; expand only if pagination is needed.
            let mut albums = Vec::new();
            for item in items {
                let Some(endpoint) =
                    item.pointer("/musicTwoRowItemRenderer/navigationEndpoint/browseEndpoint")
                else {
                    continue;
                };
                if endpoint.pointer("/browseEndpointContextSupportedConfigs/browseEndpointContextMusicConfig/pageType").and_then(Value::as_str) != Some("MUSIC_PAGE_TYPE_ALBUM") { continue; }
                if let Some(id) = endpoint.get("browseId").and_then(Value::as_str) {
                    if !albums.contains(&id) {
                        albums.push(id);
                    }
                }
                if albums.len() == 3 {
                    break;
                }
            }
            for id in albums {
                if shelf.items.len() >= 24 {
                    break;
                }
                match browse(&client, id).await {
                    Ok(album) => {
                        let previous = shelf.items.len();
                        append_album_tracks(&album, &mut shelf.items);
                        if previous == shelf.items.len() && shelf.items.is_empty() {
                            shelf.error =
                                Some("A new release did not provide playable songs".into());
                        }
                    }
                    Err(error) => shelf.error = Some(error),
                }
            }
        }
        if shelf.items.is_empty() && shelf.error.is_none() {
            shelf.error = Some("No playable songs available in this category".into());
        }
    }
    Ok(shelves)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_categories_keep_song_identity_and_exclude_recommendations() {
        let song = json!({"musicResponsiveListItemRenderer": {
            "playlistItemData": {"videoId": "actual-song"},
            "flexColumns": [{"musicResponsiveListItemFlexColumnRenderer": {
                "text": {"runs": [{"text": "Actual song"}]}
            }}]
        }});
        let mut unrelated = song.clone();
        unrelated["musicResponsiveListItemRenderer"]["playlistItemData"]["videoId"] =
            json!("unrelated-song");
        let carousel = |title: &str, browse_id: &str, contents: Value| {
            json!({
                "musicCarouselShelfRenderer": {
                    "header": {"musicCarouselShelfBasicHeaderRenderer": {"title": {"runs": [{
                        "text": title, "navigationEndpoint": {"browseEndpoint": {"browseId": browse_id}}
                    }]}}}, "contents": contents
                }
            })
        };
        let value = json!({"contents": {"singleColumnBrowseResultsRenderer": {"tabs": [{
            "tabRenderer": {"content": {"sectionListRenderer": {"contents": [
                carousel("Recommended", "unrelated", json!([unrelated.clone()])),
                carousel("Trending", "VLsource", json!([song.clone(), song.clone(),
                    {"nestedRecommendation": unrelated.clone()},
                    {"musicTwoRowItemRenderer": {"title": {"runs": [{"text": "Album"}]},
                        "navigationEndpoint": {"browseEndpoint": {"browseId": "MPREalbum"}}}}
                ])),
                carousel("New albums & singles", "FEmusic_new_releases_albums", json!([]))
            ]}}}
        }]}}});
        let trending = source_shelf(&value, "trending").unwrap();
        let mut tracks = Vec::new();
        append_tracks(trending, &mut tracks);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].id, "actual-song");
        assert_eq!(tracks[0].title, "Actual song");
        assert!(source_shelf(&value, "new-releases").unwrap().is_empty());
        let album = json!({"contents": {"twoColumnBrowseResultsRenderer": {
            "secondaryContents": {"sectionListRenderer": {"contents": [
                {"musicShelfRenderer": {"contents": [song.clone()]}},
                {"musicCarouselShelfRenderer": {"contents": [unrelated]}}
            ]}}
        }}});
        let mut album_tracks = Vec::new();
        append_album_tracks(&album, &mut album_tracks);
        assert_eq!(album_tracks.len(), 1);
        assert_eq!(album_tracks[0].id, "actual-song");
    }
}
