#[allow(dead_code)]
#[path = "../src/library.rs"]
mod track_schema;
mod library {
    pub use crate::track_schema::Track;
}
#[path = "../src/recommendations.rs"]
mod recommendations;

fn main() -> Result<(), String> {
    tauri::async_runtime::block_on(async {
        let seed = library::Track {
            id: "kXYiU_JCYtU".into(),
            title: "Linkin Park - Numb (Official Music Video)".into(),
            channel: Some("Linkin Park".into()),
        };
        let mut candidates = recommendations::radio(&seed.id).await?;
        if candidates.is_empty() {
            return Err("YouTube Music radio returned no candidates".into());
        }
        let mut recent = Vec::new();
        for _ in 0..5 {
            let Some(chosen) = recommendations::choose(&seed, candidates.clone(), &[], &recent) else {
                break;
            };
            println!("{} — {}", chosen.title, chosen.channel.as_deref().unwrap_or("Unknown artist"));
            candidates.retain(|track| track.id != chosen.id);
            recent.insert(0, chosen);
        }
        if recent.is_empty() {
            return Err("YouTube Music radio returned no eligible recommendations".into());
        }
        Ok(())
    })
}
