use serde::Serialize;
use serde_json::Value;
use std::{collections::HashMap, sync::{LazyLock, Mutex}, time::{Duration, Instant}};

const MAX_ENTRIES: usize = 128;
static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(|| Mutex::new(Cache::default()));
static NEXT_REQUEST: tokio::sync::Mutex<Option<Instant>> = tokio::sync::Mutex::const_new(None);

#[derive(Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct LyricState { track_id: String, status: &'static str, result: Option<Value>, error: Option<String> }
struct Entry {
    generation: u64, title: String, artist: String, duration: Option<f64>,
    candidates: Vec<Value>, loading: bool, error: Option<String>, expires: Instant, touched: Instant,
}
#[derive(Default)]
struct Cache { entries: HashMap<String, Entry>, generation: u64, active: String }
impl Cache {
    fn begin(&mut self, id: &str, title: &str, artist: &str, now: Instant) -> Option<u64> {
        if let Some(entry) = self.entries.get_mut(id) {
            if entry.title == title && entry.artist == artist && (entry.loading || entry.expires > now) {
                entry.touched = now;
                return None;
            }
        }
        if self.entries.len() >= MAX_ENTRIES && !self.entries.contains_key(id) {
            let oldest = self.entries.iter().filter(|(_, e)| !e.loading).min_by_key(|(_, e)| e.touched).map(|(id, _)| id.clone());
            if let Some(oldest) = oldest { self.entries.remove(&oldest); } else { return None; }
        }
        self.generation += 1;
        self.entries.insert(id.into(), Entry { generation: self.generation, title: title.into(), artist: artist.into(), duration: None, candidates: vec![], loading: true, error: None, expires: now, touched: now });
        Some(self.generation)
    }
    fn finish(&mut self, id: &str, generation: u64, result: Result<Vec<Value>, String>, now: Instant) -> bool {
        let Some(entry) = self.entries.get_mut(id).filter(|e| e.generation == generation) else { return false; };
        entry.loading = false;
        match result { Ok(results) => entry.candidates = results, Err(error) => entry.error = Some(error) }
        let seconds = if entry.error.is_some() { 60 } else if best_search_match(&entry.candidates, &entry.title, &entry.artist, entry.duration).is_none() { 300 } else { 86400 };
        entry.expires = now + Duration::from_secs(seconds);
        true
    }
}

fn folded(value: &str) -> String { value.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase() }
fn normalize_metadata(title: &str, artist: &str) -> (String, String) {
    let mut artist = artist.trim().to_string();
    for suffix in [" - Topic", "VEVO"] {
        if artist.ends_with(suffix) { artist.truncate(artist.len() - suffix.len()); }
    }
    let artist = artist.trim().to_string();
    let mut title = title.trim().to_string();
    loop {
        let lower = title.to_lowercase();
        let suffix = ["(official music video)", "[official music video]", "(official video)", "[official video]", "(official audio)", "[official audio]", "(lyric video)", "[lyric video]", "(lyrics)", "[lyrics]"].into_iter().find(|suffix| lower.ends_with(suffix));
        let Some(suffix) = suffix else { break; };
        title.truncate(title.len() - suffix.len());
        title = title.trim().to_string();
    }
    if let Some((prefix, song)) = title.split_once(" - ") {
        if !artist.is_empty() && folded(prefix) == folded(&artist) && !song.trim().is_empty() { title = song.trim().into(); }
    }
    (title, artist)
}
fn valid_duration(duration: f64) -> Option<f64> { (duration.is_finite() && (1.0..=86400.0).contains(&duration)).then_some(duration) }
fn best_search_match<'a>(results: &'a [Value], title: &str, artist: &str, duration: Option<f64>) -> Option<&'a Value> {
    let title = folded(title); let artist = folded(artist);
    results.iter().filter(|result| {
        folded(result["trackName"].as_str().unwrap_or_default()) == title
            && folded(result["artistName"].as_str().unwrap_or_default()) == artist
            && (result["instrumental"].as_bool() == Some(true) || ["syncedLyrics", "plainLyrics"].iter().any(|key| result[*key].as_str().is_some_and(|text| !text.trim().is_empty())))
            && duration.is_none_or(|duration| result["duration"].as_f64().is_none_or(|candidate| (candidate - duration).abs() <= (duration * 0.05).max(10.0)))
    }).min_by(|a, b| {
        let distance = |v: &Value| duration.map_or(0.0, |d| v["duration"].as_f64().map_or(f64::MAX, |v| (v - d).abs()));
        distance(a).total_cmp(&distance(b))
    })
}

/// Starts independently of extraction, webview visibility and lyric panel mounting.
pub fn prepare(track_id: &str, title: &str, artist: &str) {
    CACHE.lock().unwrap_or_else(|e| e.into_inner()).active = track_id.into();
    prepare_cached(track_id, title, artist);
}
fn prepare_cached(track_id: &str, title: &str, artist: &str) {
    if track_id.is_empty() || track_id.len() > 128 { return; }
    let (title, artist) = normalize_metadata(title, artist);
    let generation = CACHE.lock().unwrap_or_else(|e| e.into_inner()).begin(track_id, &title, &artist, Instant::now());
    let Some(generation) = generation else { return; };
    let id = track_id.to_string();
    tauri::async_runtime::spawn(async move {
        let result = if title.is_empty() || artist.is_empty() || title.len() > 500 || artist.len() > 500 { Err("metadata".into()) } else { tokio::time::timeout(Duration::from_secs(150), search(&title, &artist)).await.unwrap_or_else(|_| Err("cooldown".into())) };
        CACHE.lock().unwrap_or_else(|e| e.into_inner()).finish(&id, generation, result, Instant::now());
    });
}
pub fn refine(track_id: &str, duration: f64) {
    let Some(duration) = valid_duration(duration) else { return; };
    if let Some(entry) = CACHE.lock().unwrap_or_else(|e| e.into_inner()).entries.get_mut(track_id) { entry.duration = Some(duration); }
}
#[tauri::command]
pub fn prepare_lyrics(track_id: String, title: String, artist: String) { prepare_cached(&track_id, &title, &artist); }
#[tauri::command]
pub fn lyric_state(track_id: String, duration: Option<f64>) -> LyricState {
    if let Some(duration) = duration { refine(&track_id, duration); }
    let cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let mut state = LyricState { track_id: track_id.clone(), status: "idle", result: None, error: None };
    if let Some(entry) = cache.entries.get(&track_id) {
        state.result = best_search_match(&entry.candidates, &entry.title, &entry.artist, entry.duration).cloned();
        state.error = entry.error.clone();
        state.status = if entry.loading { "loading" } else if entry.error.is_some() { "error" } else if state.result.is_some() { "ready" } else { "unavailable" };
    }
    state
}
pub fn cache_metrics() -> (u64, u64) {
    let cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let bytes = cache.entries.iter().map(|(id, e)| id.len() + e.title.len() + e.artist.len() + e.candidates.iter().map(|v| v.to_string().len()).sum::<usize>()).sum::<usize>();
    (bytes as u64, cache.entries.len() as u64)
}
pub fn clear_cache() {
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let active = cache.active.clone();
    cache.entries.retain(|id, entry| entry.loading || *id == active);
}
fn retry_delay(header: Option<&str>, attempt: u32) -> Duration { Duration::from_secs(header.and_then(|v| v.parse::<u64>().ok()).unwrap_or(1 << attempt.min(5)).clamp(1, 60)) }
async fn search(title: &str, artist: &str) -> Result<Vec<Value>, String> {
    static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);
    let client = &*CLIENT;
    for attempt in 0..3 {
        // Reserve a provider slot; never hold the cache or playback lock over I/O.
        loop {
            let delay = {
                let mut next = NEXT_REQUEST.lock().await;
                let now = Instant::now();
                match *next {
                    Some(deadline) if deadline > now => Some(deadline - now),
                    _ => { *next = Some(now + Duration::from_millis(500)); None }
                }
            };
            if let Some(delay) = delay { tokio::time::sleep(delay).await; } else { break; }
        }
        let response = client.get("https://lrclib.net/api/search").header("User-Agent", "Sela/0.1.1 (By PGM Community)").timeout(Duration::from_secs(15)).query(&[("track_name",title),("artist_name",artist)]).send().await;
        let mut response = match response {
            Ok(response) => response,
            Err(_) if attempt < 2 => { tokio::time::sleep(retry_delay(None, attempt)).await; continue; }
            Err(_) => return Err("network".into()),
        };
        if matches!(response.status().as_u16(), 429 | 503) {
            let delay = retry_delay(response.headers().get("retry-after").and_then(|v|v.to_str().ok()), attempt);
            let mut next = NEXT_REQUEST.lock().await;
            let deadline = Instant::now() + delay;
            *next = Some(next.map_or(deadline, |existing| existing.max(deadline)));
            if attempt == 2 { return Err("cooldown".into()); }
            continue;
        }
        if response.status().as_u16() == 404 { return Ok(vec![]); }
        if !response.status().is_success() { return Err("provider".into()); }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| "network".to_string())? {
            if bytes.len() + chunk.len() > 2 * 1024 * 1024 { return Err("response".into()); }
            bytes.extend_from_slice(&chunk);
        }
        let mut results: Vec<Value> = serde_json::from_slice(&bytes).map_err(|_| "response".to_string())?;
        results.truncate(100);
        return Ok(results);
    }
    Err("provider".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strips_upload_labels_but_keeps_musical_versions() {
        assert_eq!(normalize_metadata("Artist - Song (Official Music Video)", "Artist - Topic"), ("Song".into(), "Artist".into()));
        assert_eq!(normalize_metadata("Song (Live)", "ArtistVEVO"), ("Song (Live)".into(), "Artist".into()));
        assert_eq!(normalize_metadata("Love - Hate", "Different Artist"), ("Love - Hate".into(), "Different Artist".into()));
    }

    #[test]
    fn duration_refines_matching_candidates_without_another_request() {
        let results = vec![json!({"trackName":"Song","artistName":"Artist","duration":166,"plainLyrics":"first"}),json!({"trackName":"Song","artistName":"Artist","duration":182,"plainLyrics":"right"})];
        assert_eq!(best_search_match(&results,"Song","Artist",None).unwrap()["plainLyrics"],"first");
        assert_eq!(best_search_match(&results,"Song","Artist",Some(180.0)).unwrap()["plainLyrics"],"right");
        assert!(best_search_match(&results,"Other","Artist",None).is_none());
        assert!(best_search_match(&results,"Song","Artist",Some(300.0)).is_none());
    }

    #[test]
    fn instrumental_and_unknown_duration_are_valid() {
        let results = vec![json!({"trackName":"Song","artistName":"Artist","instrumental":true})];
        assert!(best_search_match(&results,"Song","Artist",None).is_some());
        assert!(valid_duration(f64::NAN).is_none());
        assert!(valid_duration(0.0).is_none());
    }

    #[test]
    fn cache_reuses_inflight_and_negative_results_and_rejects_stale_generation() {
        let mut cache = Cache::default();
        let now = Instant::now();
        let generation = cache.begin("id", "Song", "Artist", now).unwrap();
        assert!(cache.begin("id", "Song", "Artist", now).is_none());
        let replacement = cache.begin("id", "Changed", "Artist", now).unwrap();
        assert!(!cache.finish("id", generation, Ok(vec![]), now));
        assert!(cache.finish("id", replacement, Ok(vec![]), now));
        assert!(cache.begin("id", "Changed", "Artist", now + Duration::from_secs(299)).is_none());
        assert!(cache.begin("id", "Changed", "Artist", now + Duration::from_secs(301)).is_some());
    }

    #[test]
    fn cache_is_bounded_and_does_not_evict_inflight_requests() {
        let mut cache = Cache::default();
        let now = Instant::now();
        for index in 0..MAX_ENTRIES { assert!(cache.begin(&index.to_string(), "Song", "Artist", now).is_some()); }
        assert!(cache.begin("overflow", "Song", "Artist", now).is_none());
        assert_eq!(cache.entries.len(), MAX_ENTRIES);
        let generation = cache.entries["0"].generation;
        cache.finish("0", generation, Ok(vec![]), now);
        assert!(cache.begin("overflow", "Song", "Artist", now).is_some());
        assert!(!cache.entries.contains_key("0"));
        assert_eq!(cache.entries.len(), MAX_ENTRIES);
    }

    #[test]
    fn provider_errors_expire_without_resetting_inflight_work() {
        let mut cache = Cache::default();
        let now = Instant::now();
        let generation = cache.begin("id", "Song", "Artist", now).unwrap();
        cache.finish("id", generation, Err("network".into()), now);
        assert!(cache.begin("id", "Song", "Artist", now + Duration::from_secs(59)).is_none());
        assert!(cache.begin("id", "Song", "Artist", now + Duration::from_secs(61)).is_some());
    }

    #[test]
    fn retry_delay_is_bounded() {
        assert_eq!(retry_delay(Some("999999"), 0), Duration::from_secs(60));
        assert_eq!(retry_delay(Some("0"), 0), Duration::from_secs(1));
        assert_eq!(retry_delay(None, 2), Duration::from_secs(4));
    }
}
