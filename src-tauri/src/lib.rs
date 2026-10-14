#[cfg(any(target_os = "android", test))]
mod native_store;
mod library;
mod queue;
mod history;
mod home;
mod preferences;
mod lyrics;
#[cfg(not(target_os = "android"))]
mod youtube;
#[cfg(not(target_os = "android"))]
mod extractor;
#[cfg(not(target_os = "android"))]
mod player;
#[cfg(not(target_os = "android"))]
mod discord;
#[cfg(not(target_os = "android"))]
mod desktop;
#[cfg(target_os = "android")]
mod mobile;
#[cfg(not(target_os = "android"))]
use desktop::*;
#[cfg(target_os = "android")]
use mobile::*;
use rusqlite::{Connection, params};
use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};
use tauri::Manager;

struct Store(Arc<Mutex<Connection>>);
#[derive(Serialize, Deserialize)]
struct Playlist { id: i64, name: String }
#[tauri::command]
fn playlists(store: tauri::State<Store>) -> Result<Vec<Playlist>, String> {
    let db = store.0.lock().map_err(|e| e.to_string())?;
    let mut statement = db.prepare("SELECT id,name FROM playlists ORDER BY id DESC").map_err(|e| e.to_string())?;
    let result = statement.query_map([], |r| Ok(Playlist { id: r.get(0)?, name: r.get(1)? })).map_err(|e| e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e| e.to_string());
    result
}
#[tauri::command]
fn create_playlist(name: String, store: tauri::State<Store>) -> Result<i64, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 120 { return Err("Nama playlist harus 1â€“120 karakter".into()); }
    let db = store.0.lock().map_err(|e| e.to_string())?;
    db.execute("INSERT INTO playlists(name) VALUES (?1)", params![name]).map_err(|e| e.to_string())?;
    Ok(db.last_insert_rowid())
}
#[tauri::command]
fn delete_playlist(id: i64, store: tauri::State<Store>) -> Result<(), String> {
    store.0.lock().map_err(|e| e.to_string())?.execute("DELETE FROM playlists WHERE id=?1", [id]).map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
fn add_track(playlist_id:i64,track:library::Track,store:tauri::State<Store>)->Result<(),String>{
    let db=store.0.lock().map_err(|e|e.to_string())?;
    library::add(&db,playlist_id,track)
}
#[tauri::command]
fn playlist_tracks(playlist_id:i64,store:tauri::State<Store>)->Result<Vec<library::Entry>,String>{
    let db=store.0.lock().map_err(|e|e.to_string())?;
    library::entries(&db,playlist_id)
}
#[tauri::command]
fn reorder_tracks(playlist_id:i64,ids:Vec<i64>,store:tauri::State<Store>)->Result<(),String>{
    let mut db=store.0.lock().map_err(|e|e.to_string())?;
    library::reorder(&mut db,playlist_id,ids)
}
#[tauri::command]
fn remove_track(entry_id:i64,store:tauri::State<Store>)->Result<(),String>{
    store.0.lock().map_err(|e|e.to_string())?.execute("DELETE FROM playlist_entries WHERE id=?1",[entry_id]).map_err(|e|e.to_string())?;Ok(())
}

#[tauri::command]
fn rename_playlist(id:i64,name:String,store:tauri::State<Store>)->Result<(),String>{
    let name=name.trim();
    if name.is_empty() || name.chars().count()>120{return Err("Nama playlist harus 1â€“120 karakter".into())}
    let changed=store.0.lock().map_err(|e|e.to_string())?.execute("UPDATE playlists SET name=?1 WHERE id=?2",params![name,id]).map_err(|e|e.to_string())?;
    if changed==0{return Err("Playlist tidak ditemukan".into())}Ok(())
}

#[tauri::command]
async fn replace_queue_from_playlist(playlist_id:i64,store:tauri::State<'_,Store>,app:tauri::AppHandle)->Result<Vec<library::Entry>,String>{
    let entries={let mut db=store.0.lock().map_err(|e|e.to_string())?;queue::replace_from_playlist(&mut db,playlist_id)?};
    sync_queue(&app,&store).await?;
    Ok(entries)
}

#[tauri::command]
fn queue_entries(store:tauri::State<Store>)->Result<Vec<library::Entry>,String>{let db=store.0.lock().map_err(|e|e.to_string())?;queue::entries(&db)}
#[tauri::command]
async fn enqueue(track:library::Track,store:tauri::State<'_,Store>,app:tauri::AppHandle)->Result<i64,String>{
    let id={let db=store.0.lock().map_err(|e|e.to_string())?;queue::append(&db,track)?};
    sync_queue(&app,&store).await?;
    Ok(id)
}
#[tauri::command]
async fn reorder_queue(ids:Vec<i64>,store:tauri::State<'_,Store>,app:tauri::AppHandle)->Result<(),String>{
    {let mut db=store.0.lock().map_err(|e|e.to_string())?;queue::reorder(&mut db,ids)?;}
    sync_queue(&app,&store).await
}
#[tauri::command]
async fn remove_queue_entry(id:i64,store:tauri::State<'_,Store>,app:tauri::AppHandle)->Result<(),String>{
    store.0.lock().map_err(|e|e.to_string())?.execute("DELETE FROM queue_entries WHERE id=?1",[id]).map_err(|e|e.to_string())?;
    sync_queue(&app,&store).await
}

#[tauri::command]
async fn select_queue_entry(id:i64,store:tauri::State<'_,Store>,app:tauri::AppHandle)->Result<(),String>{
    store.0.lock().map_err(|e|e.to_string())?.execute("UPDATE queue_state SET current_entry=?1 WHERE id=1",[id]).map_err(|e|e.to_string())?;
    sync_queue(&app,&store).await
}
#[tauri::command]
fn current_queue_entry(store:tauri::State<Store>)->Result<Option<library::Entry>,String>{
 let db=store.0.lock().map_err(|e|e.to_string())?;
 let id:Option<i64>=db.query_row("SELECT current_entry FROM queue_state WHERE id=1",[],|r|r.get(0)).map_err(|e|e.to_string())?;
 Ok(queue::entries(&db)?.into_iter().find(|entry|Some(entry.entry_id)==id))
}

async fn sync_queue(_app:&tauri::AppHandle,_store:&Store)->Result<(),String>{
    #[cfg(target_os = "android")]
    mobile::sync_queue(_app,_store).await?;
    Ok(())
}

#[tauri::command]
fn platform() -> &'static str {
    if cfg!(target_os = "android") { "android" } else { "windows" }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "android")]
    let builder = builder.plugin(mobile::init());
    let builder = builder.setup(|app| {
        let dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&dir)?;
        let db = Connection::open(dir.join("sela.sqlite"))?;
        db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS playlists(id INTEGER PRIMARY KEY, name TEXT NOT NULL);")?;
        library::initialize(&db)?;
        queue::initialize(&db)?;
        history::initialize(&db)?;
        preferences::initialize(&db)?;
        let db = Arc::new(Mutex::new(db));
        #[cfg(target_os = "android")]
        mobile::initialize(db.clone())?;
        app.manage(Store(db));
        #[cfg(not(target_os = "android"))]
        desktop::setup(app)?;
        Ok(())
    });
    #[cfg(not(target_os = "android"))]
    let builder = builder.invoke_handler(tauri::generate_handler![platform, playlists, create_playlist, delete_playlist, rename_playlist, search, play, set_paused, playback_state, seek, set_volume, add_track, playlist_tracks, reorder_tracks, remove_track, open_local, queue_entries, enqueue, replace_queue_from_playlist, reorder_queue, remove_queue_entry, select_queue_entry, current_queue_entry, history::record_listen, history::listening_history, home::music_home, lyrics::lyrics, discord::discord_connect, discord::discord_clear, discord::discord_update, preferences::music_genres, preferences::music_preferences, preferences::save_music_preferences]);
    #[cfg(target_os = "android")]
    let builder = builder.invoke_handler(tauri::generate_handler![platform, playlists, create_playlist, delete_playlist, rename_playlist, search, play, set_paused, playback_state, seek, set_volume, add_track, playlist_tracks, reorder_tracks, remove_track, open_local, queue_entries, enqueue, replace_queue_from_playlist, reorder_queue, remove_queue_entry, select_queue_entry, current_queue_entry, history::listening_history, home::music_home, lyrics::lyrics, playback_next, playback_previous, set_playback_options, preferences::music_genres, preferences::music_preferences, preferences::save_music_preferences]);
    builder.run(tauri::generate_context!()).expect("Sela gagal dijalankan");
}
