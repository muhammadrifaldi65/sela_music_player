use rusqlite::{params,Connection};
use crate::library::{Entry,Track};
pub fn initialize(db:&Connection)->rusqlite::Result<()>{db.execute_batch("CREATE TABLE IF NOT EXISTS queue_entries(id INTEGER PRIMARY KEY AUTOINCREMENT,video_id TEXT NOT NULL,title TEXT NOT NULL,channel TEXT,position INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS queue_state(id INTEGER PRIMARY KEY CHECK(id=1),current_entry INTEGER REFERENCES queue_entries(id) ON DELETE SET NULL); INSERT OR IGNORE INTO queue_state(id) VALUES(1);")}
pub fn entries(db:&Connection)->Result<Vec<Entry>,String>{
 let mut stmt=db.prepare("SELECT id,video_id,title,channel FROM queue_entries ORDER BY position,id").map_err(|e|e.to_string())?;
 let rows=stmt.query_map([],|r|Ok(Entry{entry_id:r.get(0)?,track:Track{id:r.get(1)?,title:r.get(2)?,channel:r.get(3)?}})).map_err(|e|e.to_string())?;
 rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}
pub fn append(db:&Connection,track:Track)->Result<i64,String>{
 if track.id.len()!=11 || !track.id.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'_'||b==b'-') || track.title.trim().is_empty() || track.title.len()>4096 || track.channel.as_ref().is_some_and(|s|s.len()>4096){return Err("Metadata lagu tidak valid".into())}
 db.execute("INSERT INTO queue_entries(video_id,title,channel,position) SELECT ?1,?2,?3,COALESCE(MAX(position),-1)+1 FROM queue_entries",params![track.id,track.title,track.channel]).map_err(|e|e.to_string())?;
 Ok(db.last_insert_rowid())
}
pub fn reorder(db:&mut Connection,ids:Vec<i64>)->Result<(),String>{
 let tx=db.transaction().map_err(|e|e.to_string())?;
 let mut expected:Vec<_>=entries(&tx)?.iter().map(|r|r.entry_id).collect();expected.sort_unstable();let mut actual=ids.clone();actual.sort_unstable();
 if actual!=expected{return Err("Urutan antrean tidak valid".into())}
 for (pos,id) in ids.iter().enumerate(){tx.execute("UPDATE queue_entries SET position=?1 WHERE id=?2",params![pos as i64,id]).map_err(|e|e.to_string())?;}
 tx.commit().map_err(|e|e.to_string())
}
pub fn replace_from_playlist(db:&mut Connection,playlist:i64)->Result<Vec<Entry>,String>{
 let tx=db.transaction().map_err(|e|e.to_string())?;
 let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM playlists WHERE id=?1)",[playlist],|r|r.get(0)).map_err(|e|e.to_string())?;
 if !exists{return Err("Playlist tidak ditemukan".into())}
 let populated:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM playlist_entries WHERE playlist_id=?1)",[playlist],|r|r.get(0)).map_err(|e|e.to_string())?;
 if !populated{return Err("Playlist masih kosong".into())}
 tx.execute("UPDATE queue_state SET current_entry=NULL WHERE id=1",[]).map_err(|e|e.to_string())?;
 tx.execute("DELETE FROM queue_entries",[]).map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO queue_entries(video_id,title,channel,position) SELECT video_id,title,channel,position FROM playlist_entries WHERE playlist_id=?1 ORDER BY position,id",[playlist]).map_err(|e|e.to_string())?;
 let rows=entries(&tx)?;
 tx.execute("UPDATE queue_state SET current_entry=?1 WHERE id=1",[rows[0].entry_id]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;
 Ok(rows)
}

pub fn next_index(index:Option<usize>,count:usize,repeat:&str,autoplay:bool,automatic:bool)->Option<usize>{
 let index=index?;
 if automatic && matches!(repeat,"once"|"song"){return Some(index)}
 if automatic && !autoplay && repeat!="queue"{return None}
 if index+1<count{Some(index+1)}else if repeat=="queue" && count>0{Some(0)}else{None}
}
#[cfg(test)] mod tests{
 use super::*;
 #[test]fn progression_preserves_repeat_and_manual_skip_semantics(){
  assert_eq!(next_index(Some(0),2,"off",false,true),None);
  assert_eq!(next_index(Some(0),2,"off",false,false),Some(1));
  for repeat in ["once","song"]{assert_eq!(next_index(Some(1),2,repeat,true,true),Some(1));assert_eq!(next_index(Some(1),2,repeat,true,false),None);}
  assert_eq!(next_index(Some(1),2,"queue",false,true),Some(0));
  assert_eq!(next_index(Some(1),2,"off",true,true),None);
  assert_eq!(next_index(None,0,"queue",true,true),None);
 }
 #[test]
 fn playlist_replacement_preserves_duplicates_and_rolls_back_invalid_sources(){
  let mut db=Connection::open_in_memory().unwrap();
  db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE playlists(id INTEGER PRIMARY KEY); INSERT INTO playlists VALUES(1),(2);").unwrap();
  crate::library::initialize(&db).unwrap();initialize(&db).unwrap();
  let old=append(&db,Track{id:"ccccccccccc".into(),title:"Old".into(),channel:None}).unwrap();
  db.execute("UPDATE queue_state SET current_entry=?1",[old]).unwrap();
  for id in ["aaaaaaaaaaa","bbbbbbbbbbb","aaaaaaaaaaa"]{crate::library::add(&db,1,Track{id:id.into(),title:id.into(),channel:None}).unwrap();}
  let rows=replace_from_playlist(&mut db,1).unwrap();
  assert_eq!(rows.iter().map(|r|r.track.id.as_str()).collect::<Vec<_>>(),vec!["aaaaaaaaaaa","bbbbbbbbbbb","aaaaaaaaaaa"]);
  let ids:Vec<_>=rows.iter().map(|r|r.entry_id).collect();
  assert_eq!(ids.iter().collect::<std::collections::HashSet<_>>().len(),3);
  assert!(!ids.contains(&old));
  let current=||db.query_row("SELECT current_entry FROM queue_state",[],|r|r.get::<_,i64>(0)).unwrap();
  assert_eq!(current(),ids[0]);
  let before=serde_json::to_string(&entries(&db).unwrap()).unwrap();
  for playlist in [2,999]{
   assert!(replace_from_playlist(&mut db,playlist).is_err());
   assert_eq!(serde_json::to_string(&entries(&db).unwrap()).unwrap(),before);
   assert_eq!(db.query_row("SELECT current_entry FROM queue_state",[],|r|r.get::<_,i64>(0)).unwrap(),ids[0]);
  }
  db.execute_batch("CREATE TRIGGER reject_queue_insert BEFORE INSERT ON queue_entries BEGIN SELECT RAISE(ABORT,'write failure'); END;").unwrap();
  assert!(replace_from_playlist(&mut db,1).is_err());
  assert_eq!(serde_json::to_string(&entries(&db).unwrap()).unwrap(),before);
  assert_eq!(db.query_row("SELECT current_entry FROM queue_state",[],|r|r.get::<_,i64>(0)).unwrap(),ids[0]);
 }
 #[test]fn duplicate_tracks_reorder_without_changing_current_entry(){let mut db=Connection::open_in_memory().unwrap();db.execute_batch("PRAGMA foreign_keys=ON").unwrap();initialize(&db).unwrap();let mut ids=Vec::new();for _ in 0..2{ids.push(append(&db,Track{id:"dQw4w9WgXcQ".into(),title:"Song".into(),channel:None}).unwrap())}assert_ne!(ids[0],ids[1]);db.execute("UPDATE queue_state SET current_entry=?1",[ids[0]]).unwrap();reorder(&mut db,vec![ids[1],ids[0]]).unwrap();assert_eq!(entries(&db).unwrap()[0].entry_id,ids[1]);let current:i64=db.query_row("SELECT current_entry FROM queue_state",[],|r|r.get(0)).unwrap();assert_eq!(current,ids[0]);assert!(reorder(&mut db,vec![ids[0],ids[0]]).is_err());assert_eq!(entries(&db).unwrap()[0].entry_id,ids[1]);}
}
