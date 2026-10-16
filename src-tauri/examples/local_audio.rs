#[path = "../src/player.rs"]
mod player;
fn main() -> Result<(), String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let player = player::Player::new(&root.join("native-runtime/libmpv-2.dll"))?;
    player.command(&["loadfile", "C:/Windows/Media/Windows Logon.wav"])?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        if let Some(position) = player.number("time-pos")? {
            if position > 0.25 {
                let duration = player.number("duration")?.ok_or("Missing duration")?;
                if duration <= position {
                    return Err("Invalid playback timing".into());
                }
                println!("Local audio decoded: position={position:.3}s duration={duration:.3}s");
                player.command(&["set", "pause", "yes"])?;
                return Ok(());
            }
        }
        if std::time::Instant::now() > deadline {
            return Err("Local playback did not advance".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}
