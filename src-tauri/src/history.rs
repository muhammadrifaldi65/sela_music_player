use rusqlite::Connection;
use serde::Serialize;
#[derive(Serialize)]
pub struct Listen {
    pub id: i64,
    pub video_id: String,
    pub title: String,
    pub artist: Option<String>,
    pub played_at: i64,
}
pub fn initialize(db: &Connection) -> rusqlite::Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS history(id INTEGER PRIMARY KEY,session TEXT NOT NULL UNIQUE,video_id TEXT NOT NULL,title TEXT NOT NULL,artist TEXT,played_at INTEGER NOT NULL DEFAULT(unixepoch()));")
}
#[tauri::command]
pub fn listening_history(store: tauri::State<crate::Store>) -> Result<Vec<Listen>, String> {
    let db = store.0.lock().map_err(|e| e.to_string())?;
    let mut stmt=db.prepare("SELECT id,video_id,title,artist,played_at FROM history ORDER BY played_at DESC,id DESC LIMIT 1000").map_err(|e|e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Listen {
                id: r.get(0)?,
                video_id: r.get(1)?,
                title: r.get(2)?,
                artist: r.get(3)?,
                played_at: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retries_do_not_duplicate_listens_but_replays_do() {
        let db = Connection::open_in_memory().unwrap();
        initialize(&db).unwrap();
        for session in ["first", "first", "replay"] {
            db.execute("INSERT OR IGNORE INTO history(session,video_id,title,artist) VALUES(?1,'dQw4w9WgXcQ','Song','Artist')",[session]).unwrap();
        }
        let sessions: Vec<String> = db
            .prepare("SELECT session FROM history ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(sessions, vec!["first", "replay"]);
    }
}
