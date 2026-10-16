use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use std::sync::Mutex;
pub struct Presence(pub Mutex<Option<DiscordIpcClient>>);
#[tauri::command]
pub fn discord_connect(state: tauri::State<Presence>) -> Result<(), String> {
    let mut client = DiscordIpcClient::new("1559834159294324766");
    client
        .connect()
        .map_err(|_| "Buka Discord desktop terlebih dahulu")?;
    let mut slot = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(old) = slot.as_mut() {
        let _ = old.clear_activity();
        let _ = old.close();
    }
    *slot = Some(client);
    Ok(())
}
#[tauri::command]
pub fn discord_clear(state: tauri::State<Presence>) -> Result<(), String> {
    let mut slot = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(mut client) = slot.take() {
        let result = client.clear_activity().map_err(|e| e.to_string());
        let _ = client.close();
        result
    } else {
        Ok(())
    }
}
#[tauri::command]
pub fn discord_update(
    title: String,
    artist: String,
    video_id: String,
    position: f64,
    duration: f64,
    paused: bool,
    state: tauri::State<Presence>,
    settings: tauri::State<crate::settings::SettingsState>,
) -> Result<(), String> {
    let indonesian = {
        let settings = settings.0.lock().map_err(|_| "SETTINGS_LOCK")?;
        settings.language == "id"
            || settings.language == "system" && crate::desktop::system_indonesian()
    };
    let mut slot = state.0.lock().map_err(|e| e.to_string())?;
    let client = slot.as_mut().ok_or("Discord belum terhubung")?;
    let payload = build_activity(
        title, artist, video_id, position, duration, paused, indonesian,
    )?;
    client
        .set_activity(payload)
        .map_err(|_| "Aktivitas Discord gagal diperbarui".into())
}
fn build_activity(
    title: String,
    artist: String,
    video_id: String,
    position: f64,
    duration: f64,
    paused: bool,
    indonesian: bool,
) -> Result<activity::Activity<'static>, String> {
    if title.is_empty()
        || title.len() > 512
        || artist.len() > 512
        || video_id.len() != 11
        || !video_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        || !position.is_finite()
        || !duration.is_finite()
        || position < 0.0
        || duration <= 0.0
    {
        return Err("Metadata aktivitas tidak valid".into());
    }
    let title: String = title.chars().take(128).collect();
    let artist: String = artist.chars().take(128).collect();
    let image = format!("https://i.ytimg.com/vi/{video_id}/hqdefault.jpg");
    let status = if paused {
        format!(
            "{} · {artist}",
            if indonesian { "Dijeda" } else { "Paused" }
        )
    } else {
        artist
    };
    let mut payload = activity::Activity::new()
        .activity_type(activity::ActivityType::Listening)
        .status_display_type(activity::StatusDisplayType::Details)
        .details(title.clone())
        .state(status)
        .assets(activity::Assets::new().large_image(image).large_text(title));
    if !paused {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs() as i64;
        let start = now - position as i64;
        payload = payload.timestamps(
            activity::Timestamps::new()
                .start(start)
                .end(start + duration as i64),
        );
    }
    Ok(payload)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn activity_payload_displays_song_title_in_compact_status() {
        for paused in [false, true] {
            let payload = build_activity(
                "Song".into(),
                "Artist".into(),
                "dQw4w9WgXcQ".into(),
                12.0,
                240.0,
                paused,
                false,
            )
            .unwrap();
            let json = serde_json::to_value(payload).unwrap();
            assert_eq!(json["type"], 2);
            assert_eq!(json["details"], "Song");
            assert_eq!(json["status_display_type"], 2);
            assert_eq!(
                json["assets"]["large_image"],
                "https://i.ytimg.com/vi/dQw4w9WgXcQ/hqdefault.jpg"
            );
            assert_eq!(json["assets"]["large_text"], "Song");
            if paused {
                assert!(json.get("timestamps").is_none());
            } else {
                let start = json["timestamps"]["start"].as_i64().unwrap();
                let end = json["timestamps"]["end"].as_i64().unwrap();
                assert_eq!(end - start, 240);
            }
        }
    }
}
