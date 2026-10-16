use crate::Store;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::Manager;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Appearance {
    pub accent: String,
    pub background: String,
    pub blur: f64,
    pub opacity: f64,
    pub radius: f64,
    pub glow: f64,
    pub text_size: f64,
    pub animations: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Hotkeys {
    pub enabled: bool,
    pub play_pause: String,
    pub next: String,
    pub previous: String,
    pub volume_up: String,
    pub volume_down: String,
}
impl Hotkeys {
    fn bindings(&self) -> [&str; 5] {
        [
            &self.play_pause,
            &self.next,
            &self.previous,
            &self.volume_up,
            &self.volume_down,
        ]
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppSettings {
    pub language: String,
    pub startup: bool,
    pub notifications: bool,
    pub close_to_tray: bool,
    pub record_history: bool,
    pub initial_mode: String,
    pub initial_volume: f64,
    pub appearance: Appearance,
    pub hotkeys: Hotkeys,
}
impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: "system".into(),
            startup: false,
            notifications: false,
            close_to_tray: true,
            record_history: true,
            initial_mode: "audio".into(),
            initial_volume: 80.0,
            appearance: Appearance {
                accent: "#9585ff".into(),
                background: "#1b2223".into(),
                blur: 24.0,
                opacity: 0.8,
                radius: 12.0,
                glow: 47.0,
                text_size: 16.0,
                animations: true,
            },
            hotkeys: Hotkeys {
                enabled: false,
                play_pause: "Ctrl+Alt+Space".into(),
                next: "Ctrl+Alt+ArrowRight".into(),
                previous: "Ctrl+Alt+ArrowLeft".into(),
                volume_up: "Ctrl+Alt+ArrowUp".into(),
                volume_down: "Ctrl+Alt+ArrowDown".into(),
            },
        }
    }
}
pub struct SettingsState(pub Mutex<AppSettings>);
pub fn initialize(db: &Connection) -> rusqlite::Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS app_settings(id INTEGER PRIMARY KEY CHECK(id=1),json TEXT NOT NULL);")
}
fn validate(s: &AppSettings) -> Result<(), String> {
    if !["system", "id", "en"].contains(&s.language.as_str()) {
        return Err("SETTINGS_LANGUAGE".into());
    }
    if !["audio", "video"].contains(&s.initial_mode.as_str()) {
        return Err("SETTINGS_MODE".into());
    }
    if !s.initial_volume.is_finite() || !(0.0..=100.0).contains(&s.initial_volume) {
        return Err("SETTINGS_VOLUME".into());
    }
    let a = &s.appearance;
    for color in [&a.accent, &a.background] {
        if color.len() != 7
            || !color.starts_with('#')
            || !color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
        {
            return Err("SETTINGS_APPEARANCE".into());
        }
    }
    for (value, min, max) in [
        (a.blur, 0.0, 40.0),
        (a.opacity, 0.2, 1.0),
        (a.radius, 0.0, 32.0),
        (a.glow, 0.0, 100.0),
        (a.text_size, 14.0, 20.0),
    ] {
        if !value.is_finite() || !(min..=max).contains(&value) {
            return Err("SETTINGS_APPEARANCE".into());
        }
    }
    let bindings = s.hotkeys.bindings();
    for (index, binding) in bindings.iter().enumerate() {
        if binding.len() > 80 || binding.is_empty() || bindings[..index].contains(binding) {
            return Err("SETTINGS_HOTKEY_INVALID".into());
        }
        #[cfg(not(target_os = "android"))]
        {
            binding
                .parse::<tauri_plugin_global_shortcut::Shortcut>()
                .map_err(|_| "SETTINGS_HOTKEY_INVALID")?;
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let parsed = bindings
            .iter()
            .map(|s| s.parse::<tauri_plugin_global_shortcut::Shortcut>().unwrap())
            .collect::<Vec<_>>();
        for (i, key) in parsed.iter().enumerate() {
            if parsed[..i].contains(key) {
                return Err("SETTINGS_HOTKEY_INVALID".into());
            }
        }
    }
    Ok(())
}
pub fn read_settings(db: &Connection) -> Result<AppSettings, String> {
    let raw: Option<String> = db
        .query_row("SELECT json FROM app_settings WHERE id=1", [], |r| r.get(0))
        .optional()
        .map_err(|_| "SETTINGS_READ")?;
    let settings = raw
        .map(|raw| serde_json::from_str(&raw).map_err(|_| "SETTINGS_CORRUPT"))
        .transpose()?
        .unwrap_or_default();
    validate(&settings)?;
    Ok(settings)
}
fn persist(db: &mut Connection, settings: &AppSettings) -> Result<(), String> {
    validate(settings)?;
    let raw = serde_json::to_string(settings).map_err(|_| "SETTINGS_WRITE")?;
    let tx = db.transaction().map_err(|_| "SETTINGS_WRITE")?;
    tx.execute(
        "INSERT INTO app_settings VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET json=excluded.json",
        [raw],
    )
    .map_err(|_| "SETTINGS_WRITE")?;
    tx.commit().map_err(|_| "SETTINGS_WRITE".into())
}
#[tauri::command]
pub fn get_app_settings(state: tauri::State<SettingsState>) -> Result<AppSettings, String> {
    Ok(state.0.lock().map_err(|_| "SETTINGS_LOCK")?.clone())
}
pub struct SettingsStatus(pub Mutex<Option<String>>);
#[tauri::command]
pub fn get_settings_status(state: tauri::State<SettingsStatus>) -> Result<Option<String>, String> {
    Ok(state.0.lock().map_err(|_| "SETTINGS_LOCK")?.clone())
}
#[cfg(not(target_os = "android"))]
fn register_keys(app: &tauri::AppHandle, keys: &Hotkeys) -> Result<(), String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
    if !keys.enabled {
        return Ok(());
    }
    for (action, key) in keys.bindings().iter().enumerate() {
        let shortcut = key
            .parse::<tauri_plugin_global_shortcut::Shortcut>()
            .map_err(|_| "SETTINGS_HOTKEY_INVALID")?;
        app.global_shortcut()
            .on_shortcut(shortcut, move |app, _, event| {
                if event.state == ShortcutState::Pressed {
                    crate::desktop::hotkey(app, action);
                }
            })
            .map_err(|_| "SETTINGS_HOTKEY_CONFLICT")?;
    }
    Ok(())
}
#[cfg(not(target_os = "android"))]
fn replace_keys(app: &tauri::AppHandle, from: &Hotkeys, to: &Hotkeys) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    if from == to {
        return Ok(());
    }
    app.global_shortcut()
        .unregister_all()
        .map_err(|_| "SETTINGS_HOTKEY_UNREGISTER")?;
    if let Err(error) = register_keys(app, to) {
        app.global_shortcut()
            .unregister_all()
            .map_err(|_| "SETTINGS_ROLLBACK")?;
        register_keys(app, from).map_err(|_| "SETTINGS_ROLLBACK")?;
        return Err(error);
    }
    Ok(())
}
#[tauri::command]
pub fn save_app_settings(
    settings: AppSettings,
    app: tauri::AppHandle,
    state: tauri::State<SettingsState>,
    store: tauri::State<Store>,
) -> Result<AppSettings, String> {
    validate(&settings)?;
    let mut current = state.0.lock().map_err(|_| "SETTINGS_LOCK")?;
    #[cfg(not(target_os = "android"))]
    {
        use tauri_plugin_autostart::ManagerExt;
        if settings.startup != current.startup {
            let manager = app.autolaunch();
            (if settings.startup {
                manager.enable()
            } else {
                manager.disable()
            })
            .map_err(|_| "SETTINGS_STARTUP")?;
        }
        if let Err(error) = replace_keys(&app, &current.hotkeys, &settings.hotkeys) {
            if settings.startup != current.startup {
                let manager = app.autolaunch();
                (if current.startup {
                    manager.enable()
                } else {
                    manager.disable()
                })
                .map_err(|_| "SETTINGS_ROLLBACK")?;
            }
            return Err(error);
        }
        if settings.language != current.language {
            if let Err(error) = crate::desktop::set_tray_language(&app, &settings.language) {
                crate::desktop::set_tray_language(&app, &current.language)
                    .map_err(|_| "SETTINGS_ROLLBACK")?;
                replace_keys(&app, &settings.hotkeys, &current.hotkeys)
                    .map_err(|_| "SETTINGS_ROLLBACK")?;
                if settings.startup != current.startup {
                    let manager = app.autolaunch();
                    (if current.startup {
                        manager.enable()
                    } else {
                        manager.disable()
                    })
                    .map_err(|_| "SETTINGS_ROLLBACK")?;
                }
                return Err(error);
            }
        }
    }
    let result = store
        .0
        .lock()
        .map_err(|_| "SETTINGS_LOCK".to_string())
        .and_then(|mut db| persist(&mut db, &settings));
    if let Err(error) = result {
        #[cfg(not(target_os = "android"))]
        {
            use tauri_plugin_autostart::ManagerExt;
            replace_keys(&app, &settings.hotkeys, &current.hotkeys)
                .map_err(|_| "SETTINGS_ROLLBACK")?;
            if settings.startup != current.startup {
                let manager = app.autolaunch();
                (if current.startup {
                    manager.enable()
                } else {
                    manager.disable()
                })
                .map_err(|_| "SETTINGS_ROLLBACK")?;
            }
            if settings.language != current.language {
                crate::desktop::set_tray_language(&app, &current.language)
                    .map_err(|_| "SETTINGS_ROLLBACK")?;
            }
        }
        return Err(error);
    }
    *current = settings.clone();
    if let Some(status) = app.try_state::<SettingsStatus>() {
        if let Ok(mut status) = status.0.lock() {
            *status = None;
        }
    }
    Ok(settings)
}
#[cfg(not(target_os = "android"))]
pub fn setup(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let state = app.state::<SettingsState>();
    let mut settings = state.0.lock().map_err(|_| "SETTINGS_LOCK")?;
    let error = if let Err(error) = register_keys(app, &settings.hotkeys) {
        settings.hotkeys.enabled = false;
        Some(if app.global_shortcut().unregister_all().is_err() {
            "SETTINGS_HOTKEY_UNREGISTER".into()
        } else {
            error
        })
    } else {
        None
    };
    app.manage(SettingsStatus(Mutex::new(error)));
    Ok(())
}
pub fn record_history(app: &tauri::AppHandle) -> bool {
    app.state::<SettingsState>()
        .0
        .lock()
        .map(|s| s.record_history)
        .unwrap_or(false)
}
#[cfg(not(target_os = "android"))]
pub fn notify(app: &tauri::AppHandle, title: &str, artist: &str) {
    use tauri_plugin_notification::NotificationExt;
    if app
        .state::<SettingsState>()
        .0
        .lock()
        .map(|s| s.notifications)
        .unwrap_or(false)
    {
        if let Err(error) = app
            .notification()
            .builder()
            .title(title)
            .body(artist)
            .show()
        {
            eprintln!("SETTINGS_NOTIFICATION: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_survive_restart_without_modifying_library() {
        let path = std::env::temp_dir().join(format!(
            "sela-settings-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut settings = AppSettings::default();
        settings.initial_mode = "video".into();
        settings.initial_volume = 37.0;
        settings.record_history = false;
        {
            let mut db = Connection::open(&path).unwrap();
            initialize(&db).unwrap();
            db.execute_batch(
                "CREATE TABLE playlists(name TEXT); INSERT INTO playlists VALUES('Keep');",
            )
            .unwrap();
            persist(&mut db, &settings).unwrap();
        }
        {
            let db = Connection::open(&path).unwrap();
            initialize(&db).unwrap();
            assert_eq!(read_settings(&db).unwrap(), settings);
            assert_eq!(
                db.query_row("SELECT name FROM playlists", [], |r| r.get::<_, String>(0))
                    .unwrap(),
                "Keep"
            );
        }
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn rejects_nonfinite_values_and_corrupt_storage() {
        let mut settings = AppSettings::default();
        settings.appearance.opacity = f64::NAN;
        assert!(validate(&settings).is_err());
        settings = AppSettings::default();
        settings.hotkeys.next = "not a shortcut".into();
        assert!(validate(&settings).is_err());
        let db = Connection::open_in_memory().unwrap();
        initialize(&db).unwrap();
        db.execute("INSERT INTO app_settings VALUES(1,'{}')", [])
            .unwrap();
        assert!(read_settings(&db).is_err());
    }
    #[test]
    fn persistence_validation_and_failed_write_preserve_previous_value() {
        let mut db = Connection::open_in_memory().unwrap();
        initialize(&db).unwrap();
        let mut settings = AppSettings::default();
        settings.language = "en".into();
        persist(&mut db, &settings).unwrap();
        assert_eq!(read_settings(&db).unwrap(), settings);
        let mut invalid = settings.clone();
        invalid.initial_volume = 101.0;
        assert!(persist(&mut db, &invalid).is_err());
        invalid = settings.clone();
        invalid.language = "xx".into();
        assert!(validate(&invalid).is_err());
        invalid = settings.clone();
        invalid.appearance.accent = "url(x)".into();
        assert!(validate(&invalid).is_err());
        invalid = settings.clone();
        invalid.hotkeys.next = invalid.hotkeys.previous.clone();
        assert!(validate(&invalid).is_err());
        db.execute_batch("CREATE TRIGGER fail_settings BEFORE UPDATE ON app_settings BEGIN SELECT RAISE(ABORT,'disk full'); END;").unwrap();
        let mut changed = settings.clone();
        changed.record_history = false;
        assert!(persist(&mut db, &changed).is_err());
        assert_eq!(read_settings(&db).unwrap(), settings);
    }
}
