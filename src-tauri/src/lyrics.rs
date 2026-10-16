use serde::Serialize;
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};

const MAX_ENTRIES: usize = 128;
static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(|| Mutex::new(Cache::default()));
static NEXT_REQUEST: tokio::sync::Mutex<Option<Instant>> = tokio::sync::Mutex::const_new(None);
static REQUEST_SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(6);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricState {
    track_id: String,
    status: &'static str,
    result: Option<Value>,
    error: Option<String>,
    selected_provider: String,
    resolved_provider: Option<String>,
    providers: Vec<ProviderState>,
}
const PROVIDERS: [(&str, &str); 3] = [("lrclib", "LRCLIB"), ("kugou", "KuGou"), ("lyricsplus", "LyricsPlus")];
#[derive(Clone, Serialize)]
pub struct ProviderState {
    id: String,
    name: String,
    status: &'static str,
    error: Option<String>,
    synced: bool,
}
fn provider_states(status: &'static str) -> Vec<ProviderState> {
    PROVIDERS.iter().map(|(id, name)| ProviderState {
        id: (*id).into(), name: (*name).into(), status, error: None, synced: false,
    }).collect()
}
struct Entry {
    generation: u64,
    title: String,
    artist: String,
    duration: Option<f64>,
    candidates: Vec<Value>,
    loading: bool,
    error: Option<String>,
    expires: Instant,
    touched: Instant,
    selected_provider: String,
    providers: Vec<ProviderState>,
    metadata_override: Option<(String, String)>,
}
#[derive(Default)]
struct Cache {
    entries: HashMap<String, Entry>,
    generation: u64,
    active: String,
}
impl Cache {
    fn begin(&mut self, id: &str, title: &str, artist: &str, now: Instant) -> Option<u64> {
        let metadata = self.entries.get(id).and_then(|e| e.metadata_override.clone());
        let (title, artist) = metadata.as_ref().map_or((title, artist), |(t, a)| (t.as_str(), a.as_str()));
        if let Some(entry) = self.entries.get_mut(id) {
            if entry.title == title
                && entry.artist == artist
                && (entry.loading || entry.expires > now)
            {
                entry.touched = now;
                return None;
            }
        }
        if self.entries.len() >= MAX_ENTRIES && !self.entries.contains_key(id) {
            let oldest = self
                .entries
                .iter()
                .filter(|(_, e)| !e.loading)
                .min_by_key(|(_, e)| e.touched)
                .map(|(id, _)| id.clone());
            if let Some(oldest) = oldest {
                self.entries.remove(&oldest);
            } else {
                return None;
            }
        }
        self.generation += 1;
        let previous = self.entries.remove(id);
        self.entries.insert(
            id.into(),
            Entry {
                generation: self.generation,
                title: title.into(),
                artist: artist.into(),
                duration: previous.as_ref().and_then(|e| e.duration),
                candidates: vec![],
                loading: true,
                error: None,
                expires: now,
                touched: now,
                selected_provider: previous.as_ref().map_or_else(|| "automatic".into(), |e| e.selected_provider.clone()),
                providers: provider_states("loading"),
                metadata_override: previous.and_then(|e| e.metadata_override),
            },
        );
        Some(self.generation)
    }
    fn override_metadata(&mut self, id: &str, title: &str, artist: &str, now: Instant) -> Option<u64> {
        if let Some(entry) = self.entries.get_mut(id) {
            entry.metadata_override = Some((title.into(), artist.into()));
            entry.loading = false;
            entry.expires = now;
        }
        let generation = self.begin(id, title, artist, now)?;
        self.entries.get_mut(id)?.metadata_override = Some((title.into(), artist.into()));
        Some(generation)
    }
    fn finish_provider(&mut self, id: &str, generation: u64, provider: &str, result: Result<Vec<Value>, String>, now: Instant) -> bool {
        let Some(entry) = self.entries.get_mut(id).filter(|e| e.generation == generation) else { return false; };
        let Some(state) = entry.providers.iter_mut().find(|p| p.id == provider) else { return false; };
        entry.candidates.retain(|v| v["provider"].as_str() != Some(provider));
        state.error = None;
        state.synced = false;
        match result {
            Ok(mut results) => {
                results.truncate(100);
                let matched = best_search_match(&results, &entry.title, &entry.artist, entry.duration);
                state.status = if matched.is_some() { "ready" } else { "unavailable" };
                state.synced = matched.and_then(|v| v["syncedLyrics"].as_str()).is_some_and(has_timestamps);
                for mut result in results {
                    if result.is_object() {
                        result["provider"] = provider.into();
                        entry.candidates.push(result);
                    }
                }
            }
            Err(error) => { state.status = "error"; state.error = Some(error); }
        }
        entry.loading = entry.providers.iter().any(|p| p.status == "loading");
        entry.error = entry.providers.iter().find_map(|p| p.error.clone());
        let seconds = if entry.error.is_some() { 60 } else if selected_match(entry).is_some() { 86400 } else { 300 };
        entry.expires = now + Duration::from_secs(seconds);
        true
    }
    #[cfg(test)]
    fn finish(&mut self, id: &str, generation: u64, result: Result<Vec<Value>, String>, now: Instant) -> bool {
        let accepted = self.finish_provider(id, generation, "lrclib", result, now);
        self.finish_provider(id, generation, "kugou", Ok(vec![]), now);
        self.finish_provider(id, generation, "lyricsplus", Ok(vec![]), now);
        accepted
    }
}

fn folded(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
fn normalize_metadata(title: &str, artist: &str) -> (String, String) {
    let mut artist = artist.trim().to_string();
    for suffix in [" - Topic", "VEVO"] {
        if artist.ends_with(suffix) {
            artist.truncate(artist.len() - suffix.len());
        }
    }
    let artist = artist.trim().to_string();
    let mut title = title.trim().to_string();
    loop {
        let lower = title.to_lowercase();
        let suffix = [
            "(official music video)",
            "[official music video]",
            "(official video)",
            "[official video]",
            "(official audio)",
            "[official audio]",
            "(lyric video)",
            "[lyric video]",
            "(lyrics)",
            "[lyrics]",
        ]
        .into_iter()
        .find(|suffix| lower.ends_with(suffix));
        let Some(suffix) = suffix else {
            break;
        };
        title.truncate(title.len() - suffix.len());
        title = title.trim().to_string();
    }
    if let Some((prefix, song)) = title.split_once(" - ") {
        if !artist.is_empty() && folded(prefix) == folded(&artist) && !song.trim().is_empty() {
            title = song.trim().into();
        }
    }
    (title, artist)
}
fn valid_duration(duration: f64) -> Option<f64> {
    (duration.is_finite() && (1.0..=86400.0).contains(&duration)).then_some(duration)
}
fn best_search_match<'a>(
    results: &'a [Value],
    title: &str,
    artist: &str,
    duration: Option<f64>,
) -> Option<&'a Value> {
    let title = folded(title);
    let artist = folded(artist);
    results
        .iter()
        .filter(|result| {
            folded(result["trackName"].as_str().unwrap_or_default()) == title
                && folded(result["artistName"].as_str().unwrap_or_default()) == artist
                && (result["instrumental"].as_bool() == Some(true)
                    || ["syncedLyrics", "plainLyrics"].iter().any(|key| {
                        result[*key]
                            .as_str()
                            .is_some_and(|text| !text.trim().is_empty())
                    }))
                && duration.is_none_or(|duration| {
                    result["duration"].as_f64().is_none_or(|candidate| {
                        (candidate - duration).abs() <= (duration * 0.05).max(10.0)
                    })
                })
        })
        .min_by(|a, b| {
            let synced = |v: &Value| v["syncedLyrics"].as_str().is_some_and(has_timestamps);
            if synced(a) != synced(b) {
                return synced(b).cmp(&synced(a));
            }
            let distance = |v: &Value| {
                duration.map_or(0.0, |d| {
                    v["duration"].as_f64().map_or(f64::MAX, |v| (v - d).abs())
                })
            };
            distance(a).total_cmp(&distance(b))
        })
}
fn selected_match(entry: &Entry) -> Option<&Value> {
    if entry.selected_provider == "automatic" {
        return best_search_match(&entry.candidates, &entry.title, &entry.artist, entry.duration);
    }
    entry.candidates.iter()
        .filter(|v| v["provider"].as_str() == Some(entry.selected_provider.as_str()))
        .filter(|v| best_search_match(std::slice::from_ref(*v), &entry.title, &entry.artist, entry.duration).is_some())
        .min_by(|a, b| {
            let synced = |v: &Value| v["syncedLyrics"].as_str().is_some_and(has_timestamps);
            synced(b).cmp(&synced(a)).then_with(|| {
                let distance = |v: &Value| entry.duration.map_or(0.0, |d| v["duration"].as_f64().map_or(f64::MAX, |v| (v - d).abs()));
                distance(a).total_cmp(&distance(b))
            })
        })
}
fn has_timestamps(text: &str) -> bool {
    text.lines().any(|line| {
        let Some((stamp, words)) = line.trim().strip_prefix('[').and_then(|s| s.split_once(']')) else { return false; };
        let Some((minutes, seconds)) = stamp.split_once(':') else { return false; };
        !words.trim().is_empty() && minutes.parse::<u32>().is_ok_and(|m| m <= 1440)
            && seconds.parse::<f64>().is_ok_and(|s| s.is_finite() && (0.0..60.0).contains(&s))
    })
}

/// Starts independently of extraction, webview visibility and lyric panel mounting.
pub fn prepare(track_id: &str, title: &str, artist: &str) {
    CACHE.lock().unwrap_or_else(|e| e.into_inner()).active = track_id.into();
    prepare_cached(track_id, title, artist);
}
fn prepare_cached(track_id: &str, title: &str, artist: &str) {
    if track_id.is_empty() || track_id.len() > 128 {
        return;
    }
    let (title, artist) = normalize_metadata(title, artist);
    let generation = CACHE.lock().unwrap_or_else(|e| e.into_inner()).begin(
        track_id,
        &title,
        &artist,
        Instant::now(),
    );
    let Some(generation) = generation else {
        return;
    };
    launch(track_id, generation, None);
}
fn launch(track_id: &str, generation: u64, only: Option<&str>) {
    let cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let Some(entry) = cache.entries.get(track_id).filter(|e| e.generation == generation) else { return; };
    for (provider, _) in PROVIDERS {
        if only.is_some_and(|only| only != provider) { continue; }
        let (id, title, artist) = (track_id.to_string(), entry.title.clone(), entry.artist.clone());
        tauri::async_runtime::spawn(async move {
            let result = if valid_metadata(&title, &artist) {
                tokio::time::timeout(Duration::from_secs(45), search_provider(provider, &title, &artist))
                    .await.unwrap_or_else(|_| Err("timeout".into()))
            } else { Err("metadata".into()) };
            CACHE.lock().unwrap_or_else(|e| e.into_inner()).finish_provider(&id, generation, provider, result, Instant::now());
        });
    }
}
fn valid_metadata(title: &str, artist: &str) -> bool {
    !title.trim().is_empty() && !artist.trim().is_empty() && title.len() <= 500 && artist.len() <= 500
}
#[tauri::command]
pub fn select_lyric_provider(track_id: String, provider: String, retry: Option<bool>) -> Result<(), String> {
    if provider != "automatic" && !PROVIDERS.iter().any(|(id, _)| *id == provider) { return Err("provider".into()); }
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let entry = cache.entries.get_mut(&track_id).ok_or("track")?;
    entry.selected_provider = provider.clone();
    if !retry.unwrap_or(false) { return Ok(()); }
    // Invalidate every old completion, retaining completed results from other providers.
    cache.generation += 1;
    let generation = cache.generation;
    let entry = cache.entries.get_mut(&track_id).ok_or("track")?;
    entry.generation = generation;
    let mut restart = Vec::new();
    for state in &mut entry.providers {
        if provider == "automatic" || state.id == provider || state.status == "loading" {
            state.status = "loading";
            state.error = None;
            state.synced = false;
            restart.push(state.id.clone());
        }
    }
    entry.candidates.retain(|v| !restart.iter().any(|id| v["provider"].as_str() == Some(id.as_str())));
    entry.loading = true;
    drop(cache);
    for provider in restart { launch(&track_id, generation, Some(&provider)); }
    Ok(())
}
#[tauri::command]
pub fn search_lyrics_override(track_id: String, title: String, artist: String) -> Result<(), String> {
    let (title, artist) = normalize_metadata(&title, &artist);
    if track_id.is_empty() || track_id.len() > 128 || !valid_metadata(&title, &artist) { return Err("metadata".into()); }
    let generation = CACHE.lock().unwrap_or_else(|e| e.into_inner())
        .override_metadata(&track_id, &title, &artist, Instant::now()).ok_or("busy")?;
    launch(&track_id, generation, None);
    Ok(())
}
pub fn refine(track_id: &str, duration: f64) {
    let Some(duration) = valid_duration(duration) else {
        return;
    };
    if let Some(entry) = CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .entries
        .get_mut(track_id)
    {
        entry.duration = Some(duration);
    }
}
#[tauri::command]
pub fn prepare_lyrics(track_id: String, title: String, artist: String) {
    prepare_cached(&track_id, &title, &artist);
}
#[tauri::command]
pub fn lyric_state(track_id: String, duration: Option<f64>) -> LyricState {
    if let Some(duration) = duration {
        refine(&track_id, duration);
    }
    let cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    state_for(&track_id, cache.entries.get(&track_id))
}
fn state_for(track_id: &str, entry: Option<&Entry>) -> LyricState {
    let mut state = LyricState {
        track_id: track_id.into(), status: "idle", result: None, error: None,
        selected_provider: "automatic".into(), resolved_provider: None, providers: provider_states("idle"),
    };
    if let Some(entry) = entry {
        state.selected_provider = entry.selected_provider.clone();
        state.providers = entry.providers.clone();
        for provider in &mut state.providers {
            if matches!(provider.status, "ready" | "unavailable") {
                let matched = entry.candidates.iter().filter(|v| v["provider"].as_str() == Some(provider.id.as_str()))
                    .filter(|v| best_search_match(std::slice::from_ref(*v), &entry.title, &entry.artist, entry.duration).is_some());
                let synced = matched.clone().any(|v| v["syncedLyrics"].as_str().is_some_and(has_timestamps));
                let available = matched.count() > 0;
                provider.synced = synced;
                provider.status = if available { "ready" } else { "unavailable" };
            }
        }
        state.result = selected_match(entry).cloned();
        state.resolved_provider = state.result.as_ref().and_then(|v| v["provider"].as_str()).map(str::to_string);
        let relevant = || state.providers.iter().filter(|p| entry.selected_provider == "automatic" || p.id == entry.selected_provider);
        let loading = relevant().any(|p| p.status == "loading");
        state.error = relevant().find_map(|p| p.error.clone());
        state.status = if state.result.is_some() { "ready" }
            else if loading { "loading" }
            else if state.error.is_some() { "error" } else { "unavailable" };
        if state.status != "error" { state.error = None; }
    }
    state
}

async fn provider_json(url: &str, query: &[(&str, &str)]) -> Result<Value, String> {
    static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);
    let mut response = CLIENT.get(url).query(query).header("User-Agent", "Sela/0.1.1")
        .timeout(Duration::from_secs(15)).send().await.map_err(|_| "network".to_string())?;
    if response.status().as_u16() == 404 { return Ok(Value::Null); }
    if matches!(response.status().as_u16(), 429 | 503) { return Err("cooldown".into()); }
    if !response.status().is_success() { return Err("provider".into()); }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "network".to_string())? {
        if bytes.len() + chunk.len() > 2 * 1024 * 1024 { return Err("response".into()); }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| "response".into())
}
async fn search_provider(provider: &str, title: &str, artist: &str) -> Result<Vec<Value>, String> {
    let _permit = REQUEST_SLOTS.acquire().await.map_err(|_| "busy".to_string())?;
    match provider {
        "lrclib" => search(title, artist).await,
        "kugou" => search_kugou(title, artist).await,
        "lyricsplus" => {
            let value = provider_json("https://lyricsplus.prjktla.my.id/v2/lyrics/get", &[("title", title), ("artist", artist)]).await?;
            lyricsplus_result(&value)
        }
        _ => Err("provider".into()),
    }
}
fn lyricsplus_result(value: &Value) -> Result<Vec<Value>, String> {
    if value.is_null() { return Ok(vec![]); }
    let lines = value["lyrics"].as_array().ok_or("response")?;
    let metadata = &value["metadata"];
    let title = metadata["title"].as_str().ok_or("response")?;
    let artist = metadata["artist"].as_str().ok_or("response")?;
    let synchronized = matches!(value["type"].as_str(), Some("Word" | "Line"));
    let mut plain = String::new();
    let mut lrc = String::new();
    for line in lines {
        let Some(text) = line["text"].as_str().filter(|s| !s.trim().is_empty()) else { continue; };
        if !plain.is_empty() { plain.push('\n'); }
        plain.push_str(text);
        if synchronized {
            if let Some(time) = line["time"].as_f64().filter(|t| t.is_finite() && (0.0..=86400000.0).contains(t)) {
                let centiseconds = (time / 10.0).round() as u64;
                use std::fmt::Write;
                let _ = writeln!(lrc, "[{:02}:{:02}.{:02}]{}", centiseconds / 6000, centiseconds / 100 % 60, centiseconds % 100, text);
            }
        }
    }
    Ok(vec![serde_json::json!({"trackName":title,"artistName":artist,"plainLyrics":plain,
        "syncedLyrics": if has_timestamps(&lrc) { Some(lrc) } else { None }, "instrumental":false})])
}
async fn search_kugou(title: &str, artist: &str) -> Result<Vec<Value>, String> {
    let keyword = format!("{artist} - {title}");
    let value = provider_json("https://lyrics.kugou.com/search", &[("ver", "1"), ("man", "yes"), ("keyword", &keyword)]).await?;
    if value.is_null() { return Ok(vec![]); }
    let candidates = value["candidates"].as_array().ok_or("response")?;
    let mut results = Vec::new();
    for candidate in candidates.iter().take(20) {
        let song = candidate["song"].as_str().unwrap_or_default();
        let singer = candidate["singer"].as_str().unwrap_or_default();
        if folded(song) != folded(title) || folded(singer) != folded(artist) { continue; }
        let id = candidate["id"].as_str().map(str::to_string).or_else(|| candidate["id"].as_u64().map(|id| id.to_string())).ok_or("response")?;
        let key = candidate["accesskey"].as_str().ok_or("response")?;
        let downloaded = provider_json("https://lyrics.kugou.com/download", &[("ver", "1"), ("client", "pc"), ("id", &id), ("accesskey", key), ("fmt", "lrc"), ("charset", "utf8")]).await?;
        let text = decode_lyric(downloaded["content"].as_str().ok_or("response")?)?;
        results.push(serde_json::json!({"trackName":song,"artistName":singer,
            "duration":candidate["duration"].as_f64().map(|d| d / 1000.0),
            "syncedLyrics":if has_timestamps(&text) { Some(text.clone()) } else { None },
            "plainLyrics":text,"instrumental":false}));
        if results.len() >= 3 { break; }
    }
    Ok(results)
}
fn decode_lyric(encoded: &str) -> Result<String, String> {
    let mut bytes = Vec::with_capacity(encoded.len() * 3 / 4);
    let mut bits = 0u32;
    let mut count = 0;
    let mut padding = false;
    for byte in encoded.bytes().filter(|b| !b.is_ascii_whitespace()) {
        if byte == b'=' { padding = true; continue; }
        if padding { return Err("response".into()); }
        let value = match byte {
            b'A'..=b'Z' => byte - b'A', b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52, b'+' => 62, b'/' => 63,
            _ => return Err("response".into()),
        };
        bits = (bits << 6) | u32::from(value);
        count += 6;
        if count >= 8 { count -= 8; bytes.push((bits >> count) as u8); }
    }
    if count == 6 || (count > 0 && bits & ((1 << count) - 1) != 0) { return Err("response".into()); }
    String::from_utf8(bytes).map_err(|_| "response".into())
}
pub fn cache_metrics() -> (u64, u64) {
    let cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let bytes = cache
        .entries
        .iter()
        .map(|(id, e)| {
            id.len()
                + e.title.len()
                + e.artist.len()
                + e.candidates
                    .iter()
                    .map(|v| v.to_string().len())
                    .sum::<usize>()
        })
        .sum::<usize>();
    (bytes as u64, cache.entries.len() as u64)
}
pub fn clear_cache() {
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let active = cache.active.clone();
    cache
        .entries
        .retain(|id, entry| entry.loading || *id == active);
}
fn retry_delay(header: Option<&str>, attempt: u32) -> Duration {
    Duration::from_secs(
        header
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(1 << attempt.min(5))
            .clamp(1, 60),
    )
}
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
                    _ => {
                        *next = Some(now + Duration::from_millis(500));
                        None
                    }
                }
            };
            if let Some(delay) = delay {
                tokio::time::sleep(delay).await;
            } else {
                break;
            }
        }
        let response = client
            .get("https://lrclib.net/api/search")
            .header("User-Agent", "Sela/0.1.1 (By PGM Community)")
            .timeout(Duration::from_secs(15))
            .query(&[("track_name", title), ("artist_name", artist)])
            .send()
            .await;
        let mut response = match response {
            Ok(response) => response,
            Err(_) if attempt < 2 => {
                tokio::time::sleep(retry_delay(None, attempt)).await;
                continue;
            }
            Err(_) => return Err("network".into()),
        };
        if matches!(response.status().as_u16(), 429 | 503) {
            let delay = retry_delay(
                response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok()),
                attempt,
            );
            let mut next = NEXT_REQUEST.lock().await;
            let deadline = Instant::now() + delay;
            *next = Some(next.map_or(deadline, |existing| existing.max(deadline)));
            if attempt == 2 {
                return Err("cooldown".into());
            }
            continue;
        }
        if response.status().as_u16() == 404 {
            return Ok(vec![]);
        }
        if !response.status().is_success() {
            return Err("provider".into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| "network".to_string())? {
            if bytes.len() + chunk.len() > 2 * 1024 * 1024 {
                return Err("response".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        let mut results: Vec<Value> =
            serde_json::from_slice(&bytes).map_err(|_| "response".to_string())?;
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
        assert_eq!(
            normalize_metadata("Artist - Song (Official Music Video)", "Artist - Topic"),
            ("Song".into(), "Artist".into())
        );
        assert_eq!(
            normalize_metadata("Song (Live)", "ArtistVEVO"),
            ("Song (Live)".into(), "Artist".into())
        );
        assert_eq!(
            normalize_metadata("Love - Hate", "Different Artist"),
            ("Love - Hate".into(), "Different Artist".into())
        );
    }

    #[test]
    fn duration_refines_matching_candidates_without_another_request() {
        let results = vec![
            json!({"trackName":"Song","artistName":"Artist","duration":166,"plainLyrics":"first"}),
            json!({"trackName":"Song","artistName":"Artist","duration":182,"plainLyrics":"right"}),
        ];
        assert_eq!(
            best_search_match(&results, "Song", "Artist", None).unwrap()["plainLyrics"],
            "first"
        );
        assert_eq!(
            best_search_match(&results, "Song", "Artist", Some(180.0)).unwrap()["plainLyrics"],
            "right"
        );
        assert!(best_search_match(&results, "Other", "Artist", None).is_none());
        assert!(best_search_match(&results, "Song", "Artist", Some(300.0)).is_none());
    }

    #[test]
    fn instrumental_and_unknown_duration_are_valid() {
        let results = vec![json!({"trackName":"Song","artistName":"Artist","instrumental":true})];
        assert!(best_search_match(&results, "Song", "Artist", None).is_some());
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
        assert!(cache
            .begin("id", "Changed", "Artist", now + Duration::from_secs(299))
            .is_none());
        assert!(cache
            .begin("id", "Changed", "Artist", now + Duration::from_secs(301))
            .is_some());
    }

    #[test]
    fn provider_payloads_preserve_real_timestamps_only() {
        assert_eq!(decode_lyric("WzAwOjAxLjAwXVdvcmRz").unwrap(), "[00:01.00]Words");
        assert!(decode_lyric("invalid!").is_err());
        let mut payload = json!({"type":"Line", "metadata":{"title":"Song","artist":"Artist"},
            "lyrics":[{"time":1230,"text":"Words"}]});
        let result = lyricsplus_result(&payload).unwrap();
        assert_eq!(result[0]["syncedLyrics"], "[00:01.23]Words\n");
        payload["type"] = "None".into();
        let result = lyricsplus_result(&payload).unwrap();
        assert!(result[0]["syncedLyrics"].is_null());
        assert_eq!(result[0]["plainLyrics"], "Words");
    }

    #[test]
    fn override_survives_expiry_and_preserves_duration() {
        let mut cache = Cache::default();
        let now = Instant::now();
        cache.begin("id", "Original", "Artist", now);
        cache.entries.get_mut("id").unwrap().duration = Some(180.0);
        let generation = cache.override_metadata("id", "Corrected", "Artist", now).unwrap();
        cache.finish("id", generation, Ok(vec![]), now);
        assert!(cache.begin("id", "Original", "Artist", now + Duration::from_secs(301)).is_some());
        assert_eq!(cache.entries["id"].title, "Corrected");
        assert_eq!(cache.entries["id"].duration, Some(180.0));
    }

    #[test]
    fn cache_is_bounded_and_does_not_evict_inflight_requests() {
        let mut cache = Cache::default();
        let now = Instant::now();
        for index in 0..MAX_ENTRIES {
            assert!(cache
                .begin(&index.to_string(), "Song", "Artist", now)
                .is_some());
        }
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
        assert!(cache
            .begin("id", "Song", "Artist", now + Duration::from_secs(59))
            .is_none());
        assert!(cache
            .begin("id", "Song", "Artist", now + Duration::from_secs(61))
            .is_some());
    }

    #[test]
    fn override_survives_preparation_and_stale_completions() {
        let mut cache = Cache::default();
        let now = Instant::now();
        let old = cache.begin("id", "Original", "Artist", now).unwrap();
        let generation = cache.override_metadata("id", "Corrected", "Artist", now).unwrap();
        assert!(cache.begin("id", "Original", "Artist", now).is_none());
        assert!(!cache.finish_provider("id", old, "lrclib", Ok(vec![]), now));
        assert!(cache.finish_provider("id", generation, "kugou", Ok(vec![json!({"trackName":"Corrected","artistName":"Artist","syncedLyrics":"[00:01.00]Words"})]), now));
        let entry = &cache.entries["id"];
        let state = state_for("id", Some(entry));
        assert_eq!(state.status, "ready");
        assert_eq!(state.resolved_provider.as_deref(), Some("kugou"));
        assert_eq!(state.providers[0].status, "loading");
        assert_eq!(entry.title, "Corrected");
    }

    #[test]
    fn pinned_provider_exposes_its_own_error_not_another_result() {
        let mut cache = Cache::default();
        let now = Instant::now();
        let generation = cache.begin("id", "Song", "Artist", now).unwrap();
        cache.finish_provider("id", generation, "lrclib", Err("network".into()), now);
        cache.finish_provider("id", generation, "kugou", Ok(vec![json!({"trackName":"Song","artistName":"Artist","plainLyrics":"Words"})]), now);
        cache.entries.get_mut("id").unwrap().selected_provider = "lrclib".into();
        let state = state_for("id", cache.entries.get("id"));
        assert_eq!(state.status, "error");
        assert_eq!(state.error.as_deref(), Some("network"));
        assert!(state.result.is_none());
    }
    #[test]
    fn timestamps_require_valid_time_and_text() {
        assert!(has_timestamps("[00:01.25]Hello"));
        assert!(!has_timestamps("[ar:Artist]\nJust words"));
        assert!(!has_timestamps("[00:99.00]Invalid"));
        assert!(!has_timestamps("[00:01.00]"));
    }

    #[test]
    fn automatic_prefers_synced_and_manual_stays_pinned() {
        let mut cache = Cache::default();
        let now = Instant::now();
        cache.begin("id", "Song", "Artist", now);
        let entry = cache.entries.get_mut("id").unwrap();
        entry.candidates = vec![
            json!({"provider":"lrclib","trackName":"Song","artistName":"Artist","plainLyrics":"Words"}),
            json!({"provider":"kugou","trackName":"Song","artistName":"Artist","syncedLyrics":"[00:01.00]Words"}),
        ];
        assert_eq!(selected_match(entry).unwrap()["provider"], "kugou");
        entry.selected_provider = "lrclib".into();
        assert_eq!(selected_match(entry).unwrap()["provider"], "lrclib");
        entry.selected_provider = "lyricsplus".into();
        assert!(selected_match(entry).is_none());
    }

    #[test]
    fn retry_delay_is_bounded() {
        assert_eq!(retry_delay(Some("999999"), 0), Duration::from_secs(60));
        assert_eq!(retry_delay(Some("0"), 0), Duration::from_secs(1));
        assert_eq!(retry_delay(None, 2), Duration::from_secs(4));
    }
}
