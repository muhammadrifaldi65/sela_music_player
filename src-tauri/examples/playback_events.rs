#[path = "../src/player.rs"]
mod player;
fn main() -> Result<(), String> {
    let path=std::env::args().nth(1).ok_or("Usage: cargo run --example playback_events -- <local-media-file>")?;
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let player=player::Player::new(&root.join("native-runtime/libmpv-2.dll"))?;
    player.command(&["loadfile",&path])?;
    let deadline=std::time::Instant::now()+std::time::Duration::from_secs(30);
    let mut started=None;let mut loaded=false;let mut paused=false;
    loop {
        while let Some(event)=player.event()? {match event {
            player::Event::Start(id)=>{started=Some(id);println!("start {id}");},
            player::Event::Loaded=>{loaded=true;println!("loaded");},
            player::Event::End{id,reason,error}=>{assert_eq!(Some(id),started);assert!(loaded && paused);assert_eq!(reason,0,"end error={error}");println!("natural EOF {id}; pause preserved");return Ok(())}
        }}
        if loaded && !paused && player.number("time-pos")?.unwrap_or(0.0)>0.25 {
            player.command(&["set","pause","yes"])?;
            let before=player.number("time-pos")?.unwrap();
            std::thread::sleep(std::time::Duration::from_millis(400));
            let after=player.number("time-pos")?.unwrap();assert!((after-before).abs()<0.1);
            assert!(player.flag("pause")?);player.command(&["set","pause","no"])?;paused=true;
        }
        if std::time::Instant::now()>deadline{return Err("Timed out waiting for EOF".into())}
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}
