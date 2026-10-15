use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Archive { version:u32, playlists:Vec<SavedPlaylist> }
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedPlaylist { name:String, tracks:Vec<crate::library::Track> }
#[derive(Serialize)]
pub struct ImportResult { playlists:usize, tracks:usize }
#[derive(Serialize)]
pub struct CacheMetrics { bytes:u64, files:u64 }

fn export(db:&Connection)->Result<String,String> {
    let mut statement=db.prepare("SELECT id,name FROM playlists ORDER BY id").map_err(|_|"data.database")?;
    let rows=statement.query_map([],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?))).map_err(|_|"data.database")?;
    let mut playlists=Vec::new();
    for row in rows {
        let (id,name)=row.map_err(|_|"data.database")?;
        playlists.push(SavedPlaylist{name,tracks:crate::library::entries(db,id)?.into_iter().map(|e|e.track).collect()});
    }
    serde_json::to_string_pretty(&Archive{version:1,playlists}).map_err(|_|"data.invalid_archive".into())
}
fn import(db:&mut Connection,json:&str)->Result<ImportResult,String> {
    if json.len()>16*1024*1024 {return Err("data.archive_too_large".into())}
    let archive:Archive=serde_json::from_str(json).map_err(|_|"data.invalid_archive")?;
    if archive.version!=1 || archive.playlists.len()>1000 {return Err("data.invalid_archive".into())}
    let mut tracks=0;
    for playlist in &archive.playlists {
        if playlist.name.trim().is_empty() || playlist.name.chars().count()>120 {return Err("data.invalid_archive".into())}
        tracks+=playlist.tracks.len();
        if tracks>50_000 {return Err("data.archive_too_large".into())}
        for track in &playlist.tracks {
            if track.id.len()!=11 || !track.id.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'_'||b==b'-') || track.title.trim().is_empty() || track.title.len()>4096 || track.channel.as_ref().is_some_and(|s|s.len()>4096) {return Err("data.invalid_archive".into())}
        }
    }
    let count=archive.playlists.len();
    let tx=db.transaction().map_err(|_|"data.database")?;
    for playlist in archive.playlists {
        tx.execute("INSERT INTO playlists(name) VALUES(?1)",[playlist.name.trim()]).map_err(|_|"data.database")?;
        let id=tx.last_insert_rowid();
        for (position,track) in playlist.tracks.into_iter().enumerate() {
            tx.execute("INSERT INTO playlist_entries(playlist_id,video_id,title,channel,position) VALUES(?1,?2,?3,?4,?5)",params![id,track.id,track.title,track.channel,position as i64]).map_err(|_|"data.database")?;
        }
    }
    tx.commit().map_err(|_|"data.database")?;
    Ok(ImportResult{playlists:count,tracks})
}
#[tauri::command]
pub fn export_playlists(store:tauri::State<crate::Store>)->Result<String,String> {let db=store.0.lock().map_err(|_|"data.database")?;export(&db)}
#[tauri::command]
pub fn import_playlists(json:String,store:tauri::State<crate::Store>)->Result<ImportResult,String> {let mut db=store.0.lock().map_err(|_|"data.database")?;import(&mut db,&json)}
#[tauri::command]
pub fn delete_listening_history(confirmed:bool,store:tauri::State<crate::Store>)->Result<(),String> {
    if !confirmed {return Err("data.confirmation_required".into())}
    store.0.lock().map_err(|_|"data.database")?.execute("DELETE FROM history",[]).map_err(|_|"data.database")?; Ok(())
}
#[tauri::command]
pub fn cache_metrics()->CacheMetrics {let (bytes,files)=crate::lyrics::cache_metrics();CacheMetrics{bytes,files}}
#[tauri::command]
pub fn clear_cache()->CacheMetrics {crate::lyrics::clear_cache();cache_metrics()}

#[cfg(test)]
mod tests {
    use super::*;
    fn db() -> Connection {
        let db=Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE playlists(id INTEGER PRIMARY KEY,name TEXT NOT NULL); CREATE TABLE sentinel(value TEXT); INSERT INTO sentinel VALUES('keep');").unwrap();
        crate::library::initialize(&db).unwrap(); db
    }
    #[test] fn roundtrip_preserves_duplicates_and_unrelated_data() {
        let mut db=db();
        let text=r#"{"version":1,"playlists":[{"name":"Mix","tracks":[{"id":"dQw4w9WgXcQ","title":"Song","channel":null},{"id":"dQw4w9WgXcQ","title":"Song","channel":null}]}]}"#;
        assert_eq!(import(&mut db,text).unwrap().tracks,2);
        let exported=export(&db).unwrap(); let mut other=self::db();
        assert_eq!(import(&mut other,&exported).unwrap().tracks,2);
        assert_eq!(export(&other).unwrap(),exported);
        assert_eq!(db.query_row("SELECT value FROM sentinel",[],|r|r.get::<_,String>(0)).unwrap(),"keep");
    }
    #[test] fn invalid_import_and_database_failure_roll_back() {
        let mut db=db();
        assert!(import(&mut db,r#"{"version":2,"playlists":[]}"#).is_err());
        assert!(import(&mut db,r#"{"version":1,"playlists":[{"name":"ok","tracks":[]},{"name":"bad","tracks":[{"id":"invalid","title":"Song","channel":null}]}]}"#).is_err());
        db.execute_batch("CREATE TRIGGER reject_bad BEFORE INSERT ON playlists WHEN NEW.name='bad' BEGIN SELECT RAISE(ABORT,'rejected'); END;").unwrap();
        assert!(import(&mut db,r#"{"version":1,"playlists":[{"name":"ok","tracks":[]},{"name":"bad","tracks":[]}]}"#).is_err());
        assert_eq!(db.query_row("SELECT count(*) FROM playlists",[],|r|r.get::<_,i64>(0)).unwrap(),0);
    }
}
