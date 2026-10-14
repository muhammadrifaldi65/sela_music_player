use crate::{native_store, Store};
use jni::{objects::{JClass, JString}, sys::jstring, JNIEnv};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{Manager, plugin::{Builder, PluginHandle, TauriPlugin}};

static DATABASE: OnceLock<Arc<Mutex<Connection>>> = OnceLock::new();
struct Native(PluginHandle<tauri::Wry>);

pub fn initialize(db: Arc<Mutex<Connection>>) -> Result<(), Box<dyn std::error::Error>> {
    {
        let connection = db.lock().map_err(|e| e.to_string())?;
        native_store::initialize(&connection)?;
    }
    DATABASE.set(db).map_err(|_| "Android database already initialized")?;
    Ok(())
}

pub fn init() -> TauriPlugin<tauri::Wry> {
    Builder::new("sela-playback").setup(|app, api| {
        app.manage(Native(api.register_android_plugin("app.sela.playback", "SelaPlugin")?));
        Ok(())
    }).build()
}

// Service callbacks use the same SQLite connection, without a WebView or IPC round trip.
#[no_mangle]
pub extern "system" fn Java_app_sela_playback_SelaPlugin_nativeStore(mut env: JNIEnv, _class: JClass, input: JString) -> jstring {
    let result = (|| -> Result<Value, String> {
        let text: String = env.get_string(&input).map_err(|e| e.to_string())?.into();
        if text.len() > 64 * 1024 { return Err("Native store request too large".into()); }
        let request = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let db = DATABASE.get().ok_or("Database is not ready")?.lock().map_err(|e| e.to_string())?;
        native_store::request(&db, request)
    })();
    let response = match result { Ok(value) => json!({"ok":true,"value":value}), Err(error) => json!({"ok":false,"error":error}) };
    match env.new_string(response.to_string()) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

async fn call(app: &tauri::AppHandle, command: &str, payload: Value) -> Result<Value, String> {
    app.state::<Native>().0.run_mobile_plugin_async(command, payload).await.map_err(|e| e.to_string())
}

pub async fn sync_queue(app: &tauri::AppHandle, store: &Store) -> Result<(), String> {
    let snapshot = {
        let db = store.0.lock().map_err(|e| e.to_string())?;
        native_store::request(&db, json!({"action":"snapshot"}))?
    };
    call(app,"syncQueue",json!({"snapshot":snapshot})).await?;
    Ok(())
}

#[tauri::command]
pub async fn search(query: String, app: tauri::AppHandle) -> Result<Value,String> {
    let query = query.trim();
    if query.is_empty() || query.chars().count() > 200 { return Err("Kueri pencarian tidak valid".into()); }
    let response = call(&app,"search",json!({"query":query})).await?;
    let tracks = response.get("tracks").and_then(Value::as_array).ok_or("Invalid search response")?;
    Ok(json!({"entries":tracks}))
}

#[tauri::command]
pub async fn play(video_id: String, entry_id: Option<i64>, app: tauri::AppHandle, store: tauri::State<'_,Store>) -> Result<(),String> {
    if video_id.len()!=11 || !video_id.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'_'||b==b'-') { return Err("ID video tidak valid".into()); }
    let entry = {
        let db = store.0.lock().map_err(|e|e.to_string())?;
        crate::queue::entries(&db)?.into_iter().find(|entry| entry.track.id == video_id && entry_id.is_none_or(|id|entry.entry_id==id)).ok_or("Queue entry not found")?
    };
    sync_queue(&app,&store).await?;
    call(&app,"play",json!({"track":entry.track,"entryId":entry.entry_id})).await?;
    Ok(())
}
#[tauri::command]
pub async fn set_paused(paused:bool,app:tauri::AppHandle)->Result<(),String>{call(&app,"setPaused",json!({"paused":paused})).await?;Ok(())}
#[tauri::command]
pub async fn playback_state(app:tauri::AppHandle)->Result<Value,String>{call(&app,"playbackState",json!({})).await}
#[tauri::command]
pub async fn seek(seconds:f64,app:tauri::AppHandle)->Result<(),String>{
    if !seconds.is_finite()||seconds<0.0{return Err("Posisi tidak valid".into());}
    call(&app,"seek",json!({"seconds":seconds})).await?;Ok(())
}
#[tauri::command]
pub async fn set_volume(volume:f64,app:tauri::AppHandle)->Result<(),String>{
    if !volume.is_finite()||!(0.0..=100.0).contains(&volume){return Err("Volume tidak valid".into());}
    call(&app,"setVolume",json!({"volume":volume})).await?;Ok(())
}
#[tauri::command]
pub async fn open_local(app:tauri::AppHandle)->Result<Option<String>,String>{
    let value=call(&app,"openLocal",json!({})).await?;
    Ok(value.get("title").and_then(Value::as_str).map(str::to_owned))
}
#[tauri::command]
pub async fn playback_next(app:tauri::AppHandle)->Result<(),String>{call(&app,"next",json!({})).await?;Ok(())}
#[tauri::command]
pub async fn playback_previous(app:tauri::AppHandle)->Result<(),String>{call(&app,"previous",json!({})).await?;Ok(())}
#[tauri::command]
pub async fn set_playback_options(repeat:String,autoplay:bool,app:tauri::AppHandle,store:tauri::State<'_,Store>)->Result<(),String>{
    {
        let db=store.0.lock().map_err(|e|e.to_string())?;
        native_store::request(&db,json!({"action":"options","repeat":repeat,"autoplay":autoplay}))?;
    }
    sync_queue(&app,&store).await
}
