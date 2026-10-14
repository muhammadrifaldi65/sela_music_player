use rusqlite::{params, Connection};
use serde_json::{json, Value};

pub fn initialize(db: &Connection) -> rusqlite::Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS playback_options(id INTEGER PRIMARY KEY CHECK(id=1),repeat TEXT NOT NULL DEFAULT 'off',autoplay INTEGER NOT NULL DEFAULT 0); INSERT OR IGNORE INTO playback_options(id) VALUES(1);")
}

pub fn request(db: &Connection, request: Value) -> Result<Value, String> {
    match request["action"].as_str().ok_or("Missing native store action")? {
        "snapshot" => {
            let current: Option<i64> = db.query_row("SELECT current_entry FROM queue_state WHERE id=1", [], |row| row.get(0)).map_err(|e| e.to_string())?;
            let (repeat, autoplay): (String, bool) = db.query_row("SELECT repeat,autoplay FROM playback_options WHERE id=1", [], |row| Ok((row.get(0)?, row.get(1)?))).map_err(|e| e.to_string())?;
            Ok(json!({"entries":crate::queue::entries(db)?,"current_entry":current,"repeat":repeat,"autoplay":autoplay}))
        }
        "select" | "listen" => {
            let id = request["entry_id"].as_i64().ok_or("Invalid queue entry")?;
            let exists: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM queue_entries WHERE id=?1)", [id], |row| row.get(0)).map_err(|e| e.to_string())?;
            if !exists { return Err("Queue entry no longer exists".into()); }
            if request["action"] == "select" {
                db.execute("UPDATE queue_state SET current_entry=?1 WHERE id=1", [id]).map_err(|e| e.to_string())?;
            } else {
                let session = request["session"].as_str().filter(|s| !s.is_empty() && s.len() <= 80).ok_or("Invalid listening session")?;
                db.execute("INSERT OR IGNORE INTO history(session,video_id,title,artist) SELECT ?1,video_id,title,channel FROM queue_entries WHERE id=?2", params![session,id]).map_err(|e| e.to_string())?;
            }
            Ok(Value::Null)
        }
        "append" => {
            let track = serde_json::from_value(request["track"].clone()).map_err(|e| format!("Invalid track: {e}"))?;
            Ok(json!({"entry_id":crate::queue::append(db,track)?}))
        }
        "options" => {
            let repeat = request["repeat"].as_str().filter(|v| matches!(*v,"off"|"once"|"song"|"queue")).ok_or("Invalid repeat mode")?;
            let autoplay = request["autoplay"].as_bool().ok_or("Invalid autoplay option")?;
            db.execute("UPDATE playback_options SET repeat=?1,autoplay=?2 WHERE id=1", params![repeat,autoplay]).map_err(|e|e.to_string())?;
            Ok(Value::Null)
        }
        _ => Err("Unknown native store action".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_progress_preserves_duplicate_identity_and_deduplicates_history() {
        let db = Connection::open_in_memory().unwrap();
        crate::queue::initialize(&db).unwrap();
        crate::history::initialize(&db).unwrap();
        initialize(&db).unwrap();
        let track = json!({"id":"dQw4w9WgXcQ","title":"Song","channel":null});
        let a = request(&db,json!({"action":"append","track":track})).unwrap()["entry_id"].as_i64().unwrap();
        let b = request(&db,json!({"action":"append","track":track})).unwrap()["entry_id"].as_i64().unwrap();
        assert_ne!(a,b);
        request(&db,json!({"action":"select","entry_id":b})).unwrap();
        for _ in 0..2 { request(&db,json!({"action":"listen","entry_id":b,"session":"native-session"})).unwrap(); }
        let snapshot = request(&db,json!({"action":"snapshot"})).unwrap();
        assert_eq!(snapshot["current_entry"],b);
        assert_eq!(snapshot["entries"].as_array().unwrap().len(),2);
        assert_eq!(db.query_row("SELECT count(*) FROM history",[],|r|r.get::<_,i64>(0)).unwrap(),1);
        assert!(request(&db,json!({"action":"select","entry_id":999})).is_err());
        assert!(request(&db,json!({"action":"options","repeat":"invalid","autoplay":true})).is_err());
        request(&db,json!({"action":"options","repeat":"once","autoplay":true})).unwrap();
        assert_eq!(request(&db,json!({"action":"snapshot"})).unwrap()["repeat"],"once");
    }
}
