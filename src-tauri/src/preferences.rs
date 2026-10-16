use crate::Store;
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

#[derive(Serialize)]
pub struct Genre {
    id: &'static str,
    label: &'static str,
    query: &'static str,
}

const GENRES: [Genre; 12] = [
    Genre {
        id: "pop",
        label: "Pop",
        query: "pop music official song",
    },
    Genre {
        id: "rock",
        label: "Rock",
        query: "rock music official song",
    },
    Genre {
        id: "indie",
        label: "Indie",
        query: "indie music official song",
    },
    Genre {
        id: "jazz",
        label: "Jazz",
        query: "jazz music official song",
    },
    Genre {
        id: "rnb",
        label: "R&B",
        query: "R&B music official song",
    },
    Genre {
        id: "hip-hop",
        label: "Hip-hop",
        query: "hip hop music official song",
    },
    Genre {
        id: "edm",
        label: "EDM",
        query: "EDM music official track",
    },
    Genre {
        id: "dangdut",
        label: "Dangdut",
        query: "dangdut official music video",
    },
    Genre {
        id: "k-pop",
        label: "K-pop",
        query: "k-pop official music video",
    },
    Genre {
        id: "classical",
        label: "Klasik",
        query: "classical music performance",
    },
    Genre {
        id: "reggae",
        label: "Reggae",
        query: "reggae music official song",
    },
    Genre {
        id: "metal",
        label: "Metal",
        query: "metal music official song",
    },
];

pub fn initialize(db: &Connection) -> rusqlite::Result<()> {
    // A valid singleton row is the completion marker; no separate flag can drift.
    db.execute_batch("CREATE TABLE IF NOT EXISTS music_preferences(id INTEGER PRIMARY KEY CHECK(id=1), genres TEXT NOT NULL);")
}

fn validate(genres: &[String]) -> Result<(), String> {
    if genres.is_empty() || genres.len() > GENRES.len() {
        return Err("Pilih setidaknya satu genre yang tersedia.".into());
    }
    for (index, genre) in genres.iter().enumerate() {
        if !GENRES.iter().any(|known| known.id == genre) || genres[..index].contains(genre) {
            return Err("Pilihan genre tidak valid atau berulang.".into());
        }
    }
    Ok(())
}

fn read(db: &Connection) -> Result<Option<Vec<String>>, String> {
    let raw: Option<String> = db
        .query_row(
            "SELECT genres FROM music_preferences WHERE id=1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    raw.map(|raw| {
        let genres: Vec<String> = serde_json::from_str(&raw)
            .map_err(|e| format!("Preferensi tidak dapat dibaca: {e}"))?;
        validate(&genres)?;
        Ok(genres)
    })
    .transpose()
}

fn save(db: &mut Connection, genres: Vec<String>) -> Result<Vec<String>, String> {
    validate(&genres)?;
    let raw = serde_json::to_string(&genres).map_err(|e| e.to_string())?;
    let transaction = db.transaction().map_err(|e| e.to_string())?;
    transaction.execute("INSERT INTO music_preferences(id,genres) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET genres=excluded.genres", [raw]).map_err(|e| e.to_string())?;
    transaction.commit().map_err(|e| e.to_string())?;
    Ok(genres)
}

#[tauri::command]
pub fn music_genres() -> &'static [Genre] {
    &GENRES
}

#[tauri::command]
pub fn music_preferences(store: tauri::State<Store>) -> Result<Option<Vec<String>>, String> {
    let db = store.0.lock().map_err(|e| e.to_string())?;
    read(&db)
}

#[tauri::command]
pub fn save_music_preferences(
    genres: Vec<String>,
    store: tauri::State<Store>,
) -> Result<Vec<String>, String> {
    let mut db = store.0.lock().map_err(|e| e.to_string())?;
    save(&mut db, genres)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_survives_reopen_without_touching_library() {
        let path = std::env::temp_dir().join(format!(
            "sela-preferences-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        {
            let mut db = Connection::open(&path).unwrap();
            db.execute_batch("CREATE TABLE playlists(id INTEGER PRIMARY KEY,name TEXT); INSERT INTO playlists VALUES(1,'Keep me');").unwrap();
            initialize(&db).unwrap();
            assert_eq!(read(&db).unwrap(), None);
            save(&mut db, vec!["jazz".into(), "rock".into()]).unwrap();
        }
        {
            let db = Connection::open(&path).unwrap();
            initialize(&db).unwrap();
            assert_eq!(read(&db).unwrap(), Some(vec!["jazz".into(), "rock".into()]));
            assert_eq!(
                db.query_row("SELECT name FROM playlists WHERE id=1", [], |r| r
                    .get::<_, String>(0))
                    .unwrap(),
                "Keep me"
            );
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn invalid_and_failed_saves_preserve_completed_preferences() {
        let mut db = Connection::open_in_memory().unwrap();
        initialize(&db).unwrap();
        save(&mut db, vec!["pop".into()]).unwrap();
        for invalid in [
            vec![],
            vec!["unknown".into()],
            vec!["pop".into(), "pop".into()],
        ] {
            assert!(save(&mut db, invalid).is_err());
            assert_eq!(read(&db).unwrap(), Some(vec!["pop".into()]));
        }
        db.execute_batch("CREATE TRIGGER fail_preferences BEFORE UPDATE ON music_preferences BEGIN SELECT RAISE(ABORT,'disk failure'); END;").unwrap();
        assert!(save(&mut db, vec!["metal".into()]).is_err());
        assert_eq!(read(&db).unwrap(), Some(vec!["pop".into()]));
        db.execute_batch("DROP TRIGGER fail_preferences;").unwrap();
        save(&mut db, vec!["metal".into()]).unwrap();
        assert_eq!(read(&db).unwrap(), Some(vec!["metal".into()]));
    }

    #[test]
    fn corrupt_completion_is_reported_not_silently_reset() {
        let db = Connection::open_in_memory().unwrap();
        initialize(&db).unwrap();
        db.execute(
            "INSERT INTO music_preferences VALUES(1,'[\"invented\"]')",
            [],
        )
        .unwrap();
        assert!(read(&db).is_err());
    }
}
