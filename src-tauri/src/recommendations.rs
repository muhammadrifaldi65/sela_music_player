use crate::library::Track;
use serde_json::{json, Value};
use std::time::Duration;

fn valid_id(id: &str) -> bool {
    id.len() == 11 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
fn words(text: &str) -> String {
    text.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ")
}
fn artist(text: &str) -> String {
    let mut value = words(text);
    for suffix in [" topic", "vevo"] {
        if value.ends_with(suffix) { value.truncate(value.len() - suffix.len()); }
    }
    value.trim().to_owned()
}
fn identity(track: &Track) -> (String, String) {
    let title = track.title.replace(['–', '—'], "-");
    let (performer, song) = match title.split_once(" - ") {
        Some((prefix, song)) => (artist(prefix), song),
        None => (artist(track.channel.as_deref().unwrap_or("")), title.as_str()),
    };
    let mut clean = String::new();
    let mut rest = song;
    // Only presentation/version annotations disappear; meaningful parenthesized titles stay.
    while let Some(start) = rest.find(['(', '[']) {
        clean.push_str(&rest[..start]);
        let close = if rest.as_bytes()[start] == b'(' { ')' } else { ']' };
        let Some(end) = rest[start..].find(close).map(|n| start + n) else { break; };
        let annotation = words(&rest[start + 1..end]);
        if !annotation.split_whitespace().any(|w| matches!(w, "official" | "lyrics" | "lyric" | "karaoke" | "instrumental" | "remaster" | "remastered" | "audio" | "video")) {
            clean.push_str(&rest[start..=end]);
        }
        rest = &rest[end + 1..];
    }
    clean.push_str(rest);
    let mut song = words(&clean);
    for marker in [" official", " lyrics", " lyric video", " karaoke", " instrumental", " remastered", " remaster"] {
        if let Some(index) = song.find(marker) { song.truncate(index); }
    }
    for prefix in ["karaoke ", "lyrics "] {
        if let Some(value) = song.strip_prefix(prefix) { song = value.to_owned(); }
    }
    (performer, song.trim().to_owned())
}

/// Recent listens must be newest first. Radio order breaks ties; explicit queue is never reordered.
pub fn choose(seed: &Track, candidates: Vec<Track>, excluded: &[Track], recent: &[Track]) -> Option<Track> {
    let blocked: Vec<_> = std::iter::once(seed).chain(excluded).chain(recent)
        .map(|track| (track.id.as_str(), identity(track).1)).collect();
    let artists: Vec<_> = std::iter::once(seed).chain(recent.iter().take(3))
        .map(|track| identity(track).0).filter(|name| !name.is_empty()).collect();
    let mut fallback = None;
    for track in candidates {
        if !valid_id(&track.id) || track.title.trim().is_empty() || track.title.len() > 4096
            || track.channel.as_ref().is_some_and(|channel| channel.len() > 4096) { continue; }
        let (performer, song) = identity(&track);
        if song.is_empty() || blocked.iter().any(|(id, title)| *id == track.id || *title == song) { continue; }
        if !performer.is_empty() && !artists.contains(&performer) { return Some(track); }
        if fallback.is_none() { fallback = Some(track); }
    }
    fallback
}

fn text(value: &Value) -> String {
    if let Some(text) = value["simpleText"].as_str() { return text.to_owned(); }
    value["runs"].as_array().map(|runs| runs.iter().filter_map(|run| run["text"].as_str()).collect()).unwrap_or_default()
}

pub async fn radio(seed: &str) -> Result<Vec<Track>, String> {
    if !valid_id(seed) { return Err("ID video tidak valid".into()); }
    let client = reqwest::Client::builder().timeout(Duration::from_secs(25))
        .build().map_err(|error| error.to_string())?;
    let mut response = client.post("https://music.youtube.com/youtubei/v1/next?prettyPrint=false")
        .header("Origin", "https://music.youtube.com").header("User-Agent", "Mozilla/5.0")
        .json(&json!({"context":{"client":{"clientName":"WEB_REMIX","clientVersion":"1.20261013.01.00","hl":"en","gl":"US"}},
            "videoId":seed,"playlistId":format!("RDAMVM{seed}"),"isAudioOnly":true,"enablePersistentPlaylistPanel":true}))
        .send().await.map_err(|error| error.to_string())?.error_for_status().map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
        if bytes.len() + chunk.len() > 4 * 1024 * 1024 { return Err("Respons radio terlalu besar".into()); }
        bytes.extend_from_slice(&chunk);
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "Respons radio tidak valid")?;
    let entries = value.pointer("/contents/singleColumnMusicWatchNextResultsRenderer/tabbedRenderer/watchNextTabbedResultsRenderer/tabs/0/tabRenderer/content/musicQueueRenderer/content/playlistPanelRenderer/contents")
        .and_then(Value::as_array).ok_or("Daftar radio YouTube Music tidak tersedia")?;
    Ok(entries.iter().filter_map(|entry| {
        let video = &entry["playlistPanelVideoRenderer"];
        let id = video["videoId"].as_str()?;
        let title = text(&video["title"]);
        if !valid_id(id) || title.trim().is_empty() || video["unplayableText"].is_object() { return None; }
        let channel = video["longBylineText"]["runs"].as_array().and_then(|runs| runs.iter().find(|run|
            run.pointer("/navigationEndpoint/browseEndpoint/browseEndpointContextSupportedConfigs/browseEndpointContextMusicConfig/pageType").and_then(Value::as_str) == Some("MUSIC_PAGE_TYPE_ARTIST")))
            .and_then(|run| run["text"].as_str()).map(str::to_owned)
            .or_else(|| video["shortBylineText"]["runs"][0]["text"].as_str().map(str::to_owned));
        Some(Track { id: id.to_owned(), title, channel })
    }).collect())
}

#[cfg(test)]
#[path = "recommendation_tests.rs"]
mod tests;
