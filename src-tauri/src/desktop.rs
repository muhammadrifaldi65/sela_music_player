use crate::{extractor, player, youtube, discord};
use std::sync::Mutex;
use tauri::Manager;

#[tauri::command]
pub async fn search(query: String, app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let argument = youtube::search_argument(&query)?;
    let executable = app.path().resource_dir().map_err(|e| e.to_string())?.join("yt-dlp.exe");
    let mut command = tokio::process::Command::new(executable);
    command.args(["--ignore-config", "--flat-playlist", "--dump-single-json", "--socket-timeout", "15", "--retries", "0", "--playlist-end", "20", "--", &argument]);
    command.kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let output=extractor::run(&mut command,4*1024*1024,40).await?;
    serde_json::from_slice(&output).map_err(|_| "Respons YouTube tidak valid".into())
}

#[tauri::command]
pub async fn play(video_id: String, app: tauri::AppHandle, player: tauri::State<'_, Mutex<player::Player>>, gate: tauri::State<'_, tokio::sync::Mutex<()>>) -> Result<(), String> {
    if video_id.len()!=11 || !video_id.bytes().all(|b| b.is_ascii_alphanumeric() || b==b'_' || b==b'-') { return Err("ID video tidak valid".into()); }
    let _request=gate.try_lock().map_err(|_|"Pemutar sedang menyiapkan lagu lain")?;
    let executable=app.path().resource_dir().map_err(|e|e.to_string())?.join("yt-dlp.exe");
    let mut command=tokio::process::Command::new(executable);
    let runtime=app.path().resource_dir().map_err(|e|e.to_string())?.join("node.exe");
    command.args(["--js-runtimes", &format!("node:{}",runtime.display())]);
    command.args(["--ignore-config","--no-playlist","--get-url","-f","bestaudio","--socket-timeout","15","--retries","0","--",&format!("https://www.youtube.com/watch?v={video_id}")]);
    command.kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let output=extractor::run(&mut command,64*1024,45).await?;
    let text=String::from_utf8(output).map_err(|_|"URL media tidak valid")?;
    let media=text.lines().next().ok_or("URL media kosong")?;
    if !media.starts_with("https://"){return Err("URL media tidak aman".into())}
    let player=player.lock().map_err(|e|e.to_string())?;
    player.command(&["stop"])?;
    player.command(&["loadfile",media])
}
#[tauri::command]
pub fn set_paused(paused: bool, player: tauri::State<Mutex<player::Player>>) -> Result<(),String> {
    player.lock().map_err(|e|e.to_string())?.command(&["set","pause",if paused{"yes"}else{"no"}])
}

#[tauri::command]
pub fn playback_state(player: tauri::State<Mutex<player::Player>>) -> Result<serde_json::Value,String> {
    let player=player.lock().map_err(|e|e.to_string())?;
    Ok(serde_json::json!({"position":player.number("time-pos")?,"duration":player.number("duration")?,"volume":player.number("volume")?,"paused":player.flag("pause")?,"ended":player.flag("eof-reached")?}))
}
#[tauri::command]
pub fn seek(seconds:f64, player:tauri::State<Mutex<player::Player>>) -> Result<(),String> {
    if !seconds.is_finite() || seconds < 0.0 {return Err("Posisi tidak valid".into())}
    let player=player.lock().map_err(|e|e.to_string())?;
    let duration=player.number("duration")?.ok_or("Durasi lagu belum tersedia")?;
    if seconds>duration{return Err("Posisi melebihi durasi lagu".into())}
    player.command(&["seek",&seconds.to_string(),"absolute+exact"])
}
#[tauri::command]
pub fn set_volume(volume:f64, player:tauri::State<Mutex<player::Player>>) -> Result<(),String> {
    if !volume.is_finite() || !(0.0..=100.0).contains(&volume) {return Err("Volume tidak valid".into())}
    player.lock().map_err(|e|e.to_string())?.command(&["set","volume",&volume.to_string()])
}
#[tauri::command]
pub async fn open_local(player:tauri::State<'_,Mutex<player::Player>>,gate:tauri::State<'_,tokio::sync::Mutex<()>>)->Result<Option<String>,String>{
    let Some(file)=rfd::AsyncFileDialog::new().add_filter("Audio", &["mp3","flac","wav","ogg","opus","m4a","aac","wma"]).pick_file().await else{return Ok(None)};
    let _request=gate.try_lock().map_err(|_|"Pemutar sedang menyiapkan lagu lain")?;
    let path=file.path().canonicalize().map_err(|e|e.to_string())?;
    if !path.is_file(){return Err("Berkas audio tidak ditemukan".into())}
    let value=path.to_str().ok_or("Path berkas tidak didukung")?;
    player.lock().map_err(|e|e.to_string())?.command(&["loadfile",value])?;
    Ok(Some(file.file_name()))
}

pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
        let native=player::Player::new(&app.path().resource_dir()?.join("libmpv-2.dll"))?;
        app.manage(Mutex::new(native));
        app.manage(tokio::sync::Mutex::new(()));
        app.manage(discord::Presence(Mutex::new(None)));
        let show=tauri::menu::MenuItem::with_id(app,"show","Buka Sela",true,None::<&str>)?;
        let pause=tauri::menu::MenuItem::with_id(app,"pause","Putar / jeda",true,None::<&str>)?;
        let exit=tauri::menu::MenuItem::with_id(app,"exit","Keluar",true,None::<&str>)?;
        let menu=tauri::menu::Menu::with_items(app,&[&show,&pause,&exit])?;
        let mut tray=tauri::tray::TrayIconBuilder::new().tooltip("Sela").menu(&menu).on_menu_event(|app,event|{
            match event.id.as_ref(){
                "show"=>{if let Some(window)=app.get_webview_window("main"){let _=window.show();let _=window.set_focus();}},
                "pause"=>{if let Ok(player)=app.state::<Mutex<player::Player>>().lock(){if let Err(error)=player.command(&["cycle","pause"]){eprintln!("{error}");}}},
                "exit"=>app.exit(0),
                _=>{}
            }
        });
        if let Some(icon)=app.default_window_icon(){tray=tray.icon(icon.clone());}
        tray.build(app)?;
    Ok(())
}
