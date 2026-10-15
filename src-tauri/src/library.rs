use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
pub struct Track { pub id: String, pub title: String, pub channel: Option<String> }
#[derive(Clone, Serialize)]
pub struct Entry { pub entry_id: i64, pub track: Track }
pub fn initialize(db: &Connection) -> rusqlite::Result<()> {
 db.execute_batch("CREATE TABLE IF NOT EXISTS playlist_entries(id INTEGER PRIMARY KEY, playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE, video_id TEXT NOT NULL, title TEXT NOT NULL, channel TEXT, position INTEGER NOT NULL);")
}
pub fn add(db:&Connection,playlist:i64,track:Track)->Result<(),String>{
 if track.id.len()!=11 || !track.id.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'_'||b==b'-') || track.title.trim().is_empty() || track.title.len()>4096 || track.channel.as_ref().is_some_and(|s|s.len()>4096){return Err("Metadata lagu tidak valid".into())}
 db.execute("INSERT INTO playlist_entries(playlist_id,video_id,title,channel,position) SELECT ?1,?2,?3,?4,COALESCE(MAX(position),-1)+1 FROM playlist_entries WHERE playlist_id=?1",params![playlist,track.id,track.title,track.channel]).map_err(|e|e.to_string())?;Ok(())
}
pub fn entries(db:&Connection,playlist:i64)->Result<Vec<Entry>,String>{
 let mut stmt=db.prepare("SELECT id,video_id,title,channel FROM playlist_entries WHERE playlist_id=?1 ORDER BY position,id").map_err(|e|e.to_string())?;
 let result=stmt.query_map([playlist],|r|Ok(Entry{entry_id:r.get(0)?,track:Track{id:r.get(1)?,title:r.get(2)?,channel:r.get(3)?}})).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string());result
}
pub fn reorder(db:&mut Connection,playlist:i64,ids:Vec<i64>)->Result<(),String>{
 let transaction=db.transaction().map_err(|e|e.to_string())?;
 let existing=entries(&transaction,playlist)?;
 let mut expected:Vec<_>=existing.iter().map(|e|e.entry_id).collect();expected.sort_unstable();
 let mut actual=ids.clone();actual.sort_unstable();
 if actual!=expected{return Err("Urutan harus memuat setiap lagu tepat sekali".into())}
 for (position,id) in ids.iter().enumerate(){transaction.execute("UPDATE playlist_entries SET position=?1 WHERE id=?2 AND playlist_id=?3",params![position as i64,id,playlist]).map_err(|e|e.to_string())?;}
 transaction.commit().map_err(|e|e.to_string())
}
#[cfg(test)]
mod tests{
 use super::*;
 #[test]fn repeated_tracks_keep_identity_and_invalid_reorder_is_atomic(){
 let mut db=Connection::open_in_memory().unwrap();db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE playlists(id INTEGER PRIMARY KEY); INSERT INTO playlists VALUES(1);").unwrap();initialize(&db).unwrap();
 for _ in 0..2{add(&db,1,Track{id:"dQw4w9WgXcQ".into(),title:"Song".into(),channel:None}).unwrap();}
 let rows=entries(&db,1).unwrap();let a=rows[0].entry_id;let b=rows[1].entry_id;assert_ne!(a,b);
 reorder(&mut db,1,vec![b,a]).unwrap();assert_eq!(entries(&db,1).unwrap()[0].entry_id,b);
 assert!(reorder(&mut db,1,vec![a,a]).is_err());assert_eq!(entries(&db,1).unwrap()[0].entry_id,b);
 db.execute("DELETE FROM playlists WHERE id=1",[]).unwrap();assert!(entries(&db,1).unwrap().is_empty());
 }
}
