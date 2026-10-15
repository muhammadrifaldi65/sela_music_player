use crate::{discord, extractor, library::{Entry, Track}, player, queue, video_surface, youtube, Store};
use std::{sync::Mutex, time::{Duration, Instant}};
use tauri::Manager;

#[derive(Clone)]
struct Source { video: String, audio: Option<String> }
struct Prepared { id: String, mode: String, source: Source, at: Instant }
struct Playback {
    generation: u64, active_id: Option<i64>, track: Option<Track>, entry_id: Option<i64>,
    loading: bool, paused: bool, ended: bool, error: Option<String>, repeat: String, autoplay: bool,
    mode: String, failures: u8, position: f64, duration: Option<f64>, source: Option<Source>,
    local: bool, prepared: Option<Prepared>, preparing: Option<(u64,String,String)>,
    recorded: bool, rollback: Option<(String,Source,f64)>,
    consume_once: bool, history_revision:u64, listening_session:u64,
}
impl Default for Playback {
    fn default()->Self { Self { generation:0, active_id:None, track:None, entry_id:None, loading:false, paused:true, ended:false, error:None, repeat:"off".into(), autoplay:true, mode:"audio".into(), failures:0, position:0.0, duration:None, source:None, local:false, prepared:None, preparing:None, recorded:false, rollback:None, consume_once:false, history_revision:0, listening_session:0 } }
}
impl Playback {
    fn accept_end(&mut self,id:i64)->bool { if self.active_id!=Some(id){return false} self.active_id=None; true }
    fn retry_allowed(&self)->bool { !self.paused && !self.local && self.failures<2 }
}
struct Engine { player: player::Player, state: Playback, surface: isize, bounds: (f64,f64,f64,f64,bool) }
fn lock(app:&tauri::AppHandle)->Result<tauri::State<'_,Mutex<Engine>>,String>{Ok(app.state::<Mutex<Engine>>())}
fn premature_end(position:f64,duration:Option<f64>)->bool { duration.is_some_and(|duration|duration-position>2.0) }
fn restart_previous(position:f64)->bool { position>3.0 }
fn valid_id(id:&str)->bool {id.len()==11 && id.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'_'||b==b'-')}

#[tauri::command]
pub async fn search(query:String,app:tauri::AppHandle)->Result<serde_json::Value,String>{
    let argument=youtube::search_argument(&query)?;
    let executable=app.path().resource_dir().map_err(|e|e.to_string())?.join("yt-dlp.exe");
    let mut command=tokio::process::Command::new(executable);
    command.args(["--ignore-config","--flat-playlist","--dump-single-json","--socket-timeout","15","--retries","0","--playlist-end","20","--",&argument]);
    command.kill_on_drop(true);
    #[cfg(windows)] command.creation_flags(0x08000000);
    let output=extractor::run(&mut command,4*1024*1024,40).await?;
    serde_json::from_slice(&output).map_err(|_|"Respons YouTube tidak valid".into())
}
async fn extract(app:&tauri::AppHandle,id:&str,mode:&str)->Result<Source,String>{
    let dir=app.path().resource_dir().map_err(|e|e.to_string())?;
    let mut command=tokio::process::Command::new(dir.join("yt-dlp.exe"));
    command.args(["--js-runtimes",&format!("node:{}",dir.join("node.exe").display()),"--ignore-config","--no-playlist","--get-url","-f",if mode=="video"{"bestvideo[height<=1080]+bestaudio/best[height<=1080]"}else{"bestaudio"},"--socket-timeout","15","--retries","0","--",&format!("https://www.youtube.com/watch?v={id}")]);
    command.kill_on_drop(true);
    #[cfg(windows)] command.creation_flags(0x08000000);
    let bytes=extractor::run(&mut command,64*1024,45).await?;
    let text=String::from_utf8(bytes).map_err(|_|"URL media tidak valid")?;
    let urls:Vec<_>=text.lines().filter(|line|!line.is_empty()).collect();
    if urls.is_empty() || urls.len()>2 || urls.iter().any(|url|!url.starts_with("https://") || url.chars().any(char::is_control)){return Err("URL media tidak aman".into())}
    Ok(Source{video:urls[0].into(),audio:urls.get(1).map(|s|(*s).into())})
}
fn load(engine:&mut Engine,source:&Source,position:f64)->Result<(),String>{
    engine.player.command(&["stop"])?;
    while engine.player.event()?.is_some() {}
    engine.state.active_id=None;
    engine.player.command(&["set","vid",if engine.state.mode=="video"{"auto"}else{"no"}])?;
    engine.player.command(&["set","pause",if engine.state.paused{"yes"}else{"no"}])?;
    let start=format!("start={position}");
    // audio-files is a per-file option; it cannot leak into the next audio-only load.
    let options=if let Some(audio)=&source.audio {format!("{start},audio-files=%{}%{}",audio.len(),audio)}else{start};
    engine.player.command(&["loadfile",&source.video,"replace","-1",&options])?;
    engine.state.active_id=engine.player.number("playlist/0/id")?.map(|id|id as i64);
    engine.state.source=Some(source.clone()); engine.state.loading=true; engine.state.ended=false;
    Ok(())
}
fn select(app:&tauri::AppHandle,track:Track,entry_id:Option<i64>,unpause:bool,expected:Option<u64>)->Result<(u64,String,Option<Source>),String>{
    let managed=lock(app)?;let mut engine=managed.lock().map_err(|e|e.to_string())?;
    if expected.is_some_and(|generation|generation!=engine.state.generation) || expected.is_some() && !unpause && engine.state.paused {return Err("Pilihan lagu sudah berubah".into())}
    let store=app.state::<Store>();let db=store.0.lock().map_err(|e|e.to_string())?;
    if let Some(id)=entry_id {let exists:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM queue_entries WHERE id=?1 AND video_id=?2)",rusqlite::params![id,track.id],|r|r.get(0)).map_err(|e|e.to_string())?;if !exists{return Err("Lagu tidak lagi ada di antrean".into())}}
    db.execute("UPDATE queue_state SET current_entry=?1 WHERE id=1",[entry_id]).map_err(|e|e.to_string())?;
    engine.player.command(&["stop"])?;
    let state=&mut engine.state;
    state.generation+=1;state.active_id=None;state.loading=true;state.error=None;state.ended=false;state.failures=0;state.position=0.0;state.duration=None;state.local=false;state.source=None;
    state.listening_session+=1;
    state.recorded=false;state.rollback=None;state.consume_once=false;
    if unpause {state.paused=false;}
    let cached=state.prepared.take().filter(|p|p.id==track.id && p.mode==state.mode && p.at.elapsed()<Duration::from_secs(120)).map(|p|p.source);
    crate::lyrics::prepare(&track.id,&track.title,track.channel.as_deref().unwrap_or(""));
    state.track=Some(track);state.entry_id=entry_id;state.preparing=None;
    Ok((state.generation,state.mode.clone(),cached))
}
async fn finish_load(app:&tauri::AppHandle,generation:u64,id:String,mode:String,cached:Option<Source>,position:f64)->Result<(),String>{
    let mut cached=cached;
    for attempt in 0..3 {
        if attempt>0 {tokio::time::sleep(Duration::from_secs(attempt*2)).await;}
        {let managed=lock(app)?;let engine=managed.lock().map_err(|e|e.to_string())?;
            if engine.state.generation!=generation{return Ok(())}
            if attempt>0 && engine.state.paused{return Err(engine.state.error.clone().unwrap_or_else(||"Pemulihan dibatalkan saat jeda".into()))}
        }
        let result=match cached.take() {Some(source)=>Ok(source),None=>extract(app,&id,&mode).await};
        let managed=lock(app)?;let mut engine=managed.lock().map_err(|e|e.to_string())?;
        if engine.state.generation!=generation{return Ok(())}
        let result=result.and_then(|source|load(&mut engine,&source,position));
        match result {
            Ok(())=>{engine.state.error=None;return Ok(())},
            Err(error)=>{engine.state.loading=false;engine.state.error=Some(error.clone());engine.state.prepared=None;
                if attempt==2 || engine.state.paused{return Err(error)}
            }
        }
    }
    unreachable!()
}
#[tauri::command]
pub async fn play(video_id:String,entry_id:Option<i64>,track:Option<Track>,app:tauri::AppHandle)->Result<(),String>{
    if !valid_id(&video_id){return Err("ID video tidak valid".into())}
    let track=track.unwrap_or(Track{id:video_id.clone(),title:video_id.clone(),channel:None});
    if track.id!=video_id || track.title.trim().is_empty() || track.title.len()>4096 || track.channel.as_ref().is_some_and(|c|c.len()>4096){return Err("Metadata lagu tidak valid".into())}
    let (generation,mode,cached)=select(&app,track,entry_id,true,None)?;
    finish_load(&app,generation,video_id,mode,cached,0.0).await
}
#[tauri::command]
pub fn set_paused(paused:bool,app:tauri::AppHandle)->Result<(),String>{
    let managed=lock(&app)?;let mut engine=managed.lock().map_err(|e|e.to_string())?;
    engine.player.command(&["set","pause",if paused{"yes"}else{"no"}])?;engine.state.paused=paused;Ok(())
}
#[tauri::command]
pub fn playback_state(app:tauri::AppHandle)->Result<serde_json::Value,String>{
    let managed=lock(&app)?;let engine=managed.lock().map_err(|e|e.to_string())?;let s=&engine.state;
    Ok(serde_json::json!({"position":engine.player.number("time-pos")?.unwrap_or(s.position),"duration":engine.player.number("duration")?.or(s.duration),"volume":engine.player.number("volume")?,"paused":s.paused,"ended":s.ended,"track":s.track,"entryId":s.entry_id,"loading":s.loading,"error":s.error,"repeat":s.repeat,"autoplay":s.autoplay,"mode":s.mode,"generation":s.generation,"historyRevision":s.history_revision}))
}
#[tauri::command]
pub fn seek(seconds:f64,app:tauri::AppHandle)->Result<(),String>{
    if !seconds.is_finite() || seconds<0.0{return Err("Posisi tidak valid".into())}
    let managed=lock(&app)?;let mut engine=managed.lock().map_err(|e|e.to_string())?;
    let duration=engine.player.number("duration")?.ok_or("Durasi lagu belum tersedia")?;
    if seconds>duration{return Err("Posisi melebihi durasi lagu".into())}
    engine.player.command(&["seek",&seconds.to_string(),"absolute+exact"])?;engine.state.position=seconds;Ok(())
}
#[tauri::command]
pub fn set_volume(volume:f64,app:tauri::AppHandle)->Result<(),String>{
    if !volume.is_finite() || !(0.0..=100.0).contains(&volume){return Err("Volume tidak valid".into())}
    lock(&app)?.lock().map_err(|e|e.to_string())?.player.command(&["set","volume",&volume.to_string()])
}
#[tauri::command]
pub fn set_playback_options(repeat:String,autoplay:bool,app:tauri::AppHandle)->Result<(),String>{
    if !["off","once","song","queue"].contains(&repeat.as_str()){return Err("Mode repeat tidak valid".into())}
    let managed=lock(&app)?;let mut engine=managed.lock().map_err(|e|e.to_string())?;
    app.state::<Store>().0.lock().map_err(|e|e.to_string())?.execute("UPDATE desktop_playback_options SET repeat=?1,autoplay=?2 WHERE id=1",rusqlite::params![repeat,autoplay]).map_err(|e|e.to_string())?;
    engine.state.repeat=repeat;engine.state.autoplay=autoplay;Ok(())
}
async fn advance(app:&tauri::AppHandle,previous:bool,automatic:bool,expected:Option<u64>)->Result<(),String>{
    let (generation,current,repeat,autoplay,seed,paused)={let managed=lock(app)?;let mut engine=managed.lock().map_err(|e|e.to_string())?;
        if !automatic {engine.state.generation+=1;engine.state.loading=false;engine.state.preparing=None;}
        let s=&engine.state;(s.generation,s.entry_id,s.repeat.clone(),s.autoplay,s.track.clone(),s.paused)};
    if expected.is_some_and(|expected|expected!=generation){return Ok(())}
    if automatic && paused {return Ok(())}
    let rows={let store=app.state::<Store>();let db=store.0.lock().map_err(|e|e.to_string())?;queue::entries(&db)?};
    let index=rows.iter().position(|e|Some(e.entry_id)==current);
    let chosen=if previous {index.and_then(|i|i.checked_sub(1)).and_then(|i|rows.get(i)).cloned()}
        else {queue::next_index(index,rows.len(),&repeat,autoplay,automatic).and_then(|i|rows.get(i)).cloned()};
    let chosen=if let Some(entry)=chosen {Some(entry)} else if !previous && autoplay {
        if let Some(seed)=seed {
            let candidates=crate::recommendations::radio(&seed.id).await?;
            let managed=lock(app)?;let engine=managed.lock().map_err(|e|e.to_string())?;
            if engine.state.generation!=generation || automatic && engine.state.paused{return Ok(())}
            let store=app.state::<Store>();let db=store.0.lock().map_err(|e|e.to_string())?;
            // Queue edits during the radio request still take precedence over autoplay.
            let rows=queue::entries(&db)?;
            let index=rows.iter().position(|entry|Some(entry.entry_id)==current);
            if let Some(entry)=queue::next_index(index,rows.len(),&repeat,autoplay,automatic).and_then(|i|rows.get(i)).cloned() {Some(entry)} else {
                let mut statement=db.prepare("SELECT video_id,title,artist FROM history ORDER BY played_at DESC,id DESC LIMIT 50").map_err(|e|e.to_string())?;
                let recent=statement.query_map([],|row|Ok(Track{id:row.get(0)?,title:row.get(1)?,channel:row.get(2)?})).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
                let excluded=rows.into_iter().map(|entry|entry.track).collect::<Vec<_>>();
                let track=crate::recommendations::choose(&seed,candidates,&excluded,&recent).ok_or("Tidak ada rekomendasi baru yang tersedia")?;
                let id=queue::append(&db,track.clone())?;Some(Entry{entry_id:id,track})
            }
        }else{None}
    }else{None};
    if let Some(entry)=chosen {
        {let managed=lock(app)?;let engine=managed.lock().map_err(|e|e.to_string())?;if engine.state.generation!=generation || automatic && engine.state.paused{return Ok(())}}
        let id=entry.track.id.clone();let (generation,mode,cached)=select(app,entry.track,Some(entry.entry_id),!automatic,Some(generation))?;
        if automatic && repeat=="once" {let managed=lock(app)?;let mut engine=managed.lock().map_err(|e|e.to_string())?;if engine.state.generation==generation{engine.state.consume_once=true;}}
        finish_load(app,generation,id,mode,cached,0.0).await
    }else{
        let managed=lock(app)?;let mut engine=managed.lock().map_err(|e|e.to_string())?;
        if previous {engine.player.command(&["seek","0","absolute+exact"])?;}else if automatic{engine.state.ended=true;engine.state.paused=true;}
        Ok(())
    }
}
#[tauri::command]
pub async fn playback_next(app:tauri::AppHandle)->Result<(),String>{let gate=app.state::<tokio::sync::Mutex<()>>();let _guard=gate.lock().await;advance(&app,false,false,None).await}
#[tauri::command]
pub async fn playback_previous(app:tauri::AppHandle)->Result<(),String>{
    let gate=app.state::<tokio::sync::Mutex<()>>();let _guard=gate.lock().await;
    {let managed=lock(&app)?;let mut engine=managed.lock().map_err(|e|e.to_string())?;
    if restart_previous(engine.player.number("time-pos")?.unwrap_or(engine.state.position)){engine.state.generation+=1;engine.state.loading=false;engine.player.command(&["seek","0","absolute+exact"])?;engine.state.position=0.0;return Ok(())}}
    advance(&app,true,false,None).await
}
#[tauri::command]
pub async fn set_video_mode(mode:String,app:tauri::AppHandle)->Result<(),String>{
    if !["audio","video"].contains(&mode.as_str()){return Err("Mode media tidak valid".into())}
    let (generation,id,position,old_mode)={let managed=lock(&app)?;let mut engine=managed.lock().map_err(|e|e.to_string())?;
        if mode==engine.state.mode{return Ok(())} if mode=="video" && engine.surface==0{return Err("Embedded video tidak tersedia".into())}
        if engine.state.loading{return Err("Tunggu sampai lagu selesai disiapkan".into())}
        if engine.state.local || engine.state.track.is_none(){engine.player.command(&["set","vid",if mode=="video"{"auto"}else{"no"}])?;engine.state.mode=mode;return Ok(())}
        let position=engine.player.number("time-pos")?.unwrap_or(engine.state.position);let old=engine.state.mode.clone();
        engine.state.generation+=1;engine.state.loading=true;engine.state.error=None;engine.state.prepared=None;
        (engine.state.generation,engine.state.track.as_ref().unwrap().id.clone(),position,old)
    };
    let result=extract(&app,&id,&mode).await;
    let managed=lock(&app)?;let mut engine=managed.lock().map_err(|e|e.to_string())?;
    if engine.state.generation!=generation{return Ok(())}
    let position=engine.player.number("time-pos")?.unwrap_or(position);
    match result {Ok(source)=>{engine.state.mode=mode;let old=engine.state.source.clone();engine.state.rollback=old.clone().map(|source|(old_mode.clone(),source,position));if let Err(error)=load(&mut engine,&source,position){engine.state.mode=old_mode;engine.state.rollback=None;if let Some(old)=old{let _=load(&mut engine,&old,position);}engine.state.error=Some(error.clone());return Err(error)}Ok(())},Err(error)=>{engine.state.loading=false;engine.state.error=Some(error.clone());Err(error)}}
}
#[tauri::command]
pub fn set_video_bounds(x:f64,y:f64,width:f64,height:f64,visible:bool,app:tauri::AppHandle)->Result<(),String>{
    if [x,y,width,height].iter().any(|v|!v.is_finite() || v.abs()>100_000.0) || width<0.0 || height<0.0{return Err("Batas video tidak valid".into())}
    let managed=lock(&app)?;let mut engine=managed.lock().map_err(|e|e.to_string())?;engine.bounds=(x,y,width,height,visible);
    update_surface(&app,&engine)
}
fn update_surface(app:&tauri::AppHandle,engine:&Engine)->Result<(),String>{
    if engine.surface==0{return Ok(())}
    let app_handle=app.clone();let surface=engine.surface;let(x,y,w,h,visible)=engine.bounds;let video=engine.state.mode=="video";
    app.run_on_main_thread(move || {
        let Some(window)=app_handle.get_webview_window("main") else{return};
        let Ok(scale)=window.scale_factor() else{return};
        if let Err(error)=video_surface::bounds(surface,(x*scale).round() as i32,(y*scale).round() as i32,(w*scale).round() as i32,(h*scale).round() as i32,visible && w>0.0 && h>0.0 && video && !window.is_minimized().unwrap_or(true)){eprintln!("Video surface: {error}");}
    }).map_err(|e|e.to_string())
}
#[tauri::command]
pub async fn open_local(app:tauri::AppHandle)->Result<Option<String>,String>{
    let Some(file)=rfd::AsyncFileDialog::new().add_filter("Media",&["mp3","flac","wav","ogg","opus","m4a","aac","wma","mp4","mkv","webm"]).pick_file().await else{return Ok(None)};
    let path=file.path().canonicalize().map_err(|e|e.to_string())?;if !path.is_file(){return Err("Berkas tidak ditemukan".into())}
    let source=Source{video:path.to_str().ok_or("Path tidak didukung")?.into(),audio:None};
    let managed=lock(&app)?;let mut engine=managed.lock().map_err(|e|e.to_string())?;
    engine.state.generation+=1;engine.state.track=None;engine.state.entry_id=None;engine.state.local=true;engine.state.paused=false;engine.state.error=None;engine.state.failures=0;engine.state.prepared=None;engine.state.position=0.0;engine.state.duration=None;
    engine.state.listening_session+=1;
    engine.state.rollback=None;engine.state.consume_once=false;engine.state.preparing=None;
    load(&mut engine,&source,0.0)?;Ok(Some(file.file_name()))
}

async fn worker(app:tauri::AppHandle){
    loop {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let action=(||->Result<Option<(u64,bool,String,String,f64)>,String>{
            let managed=lock(&app)?;let mut engine=managed.lock().map_err(|e|e.to_string())?;
            if let Some(position)=engine.player.number("time-pos")?{engine.state.position=position;}
            if !engine.state.recorded && !engine.state.paused && !engine.state.loading && engine.state.position>0.0 {
                if let Some(track)=&engine.state.track {
                    let session=format!("desktop-{}-{}-{}",std::process::id(),engine.state.generation,std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos());
                    let record_history=crate::settings::record_history(&app);
                    if record_history {
                        app.state::<Store>().0.lock().map_err(|e|e.to_string())?.execute("INSERT OR IGNORE INTO history(session,video_id,title,artist) VALUES(?1,?2,?3,?4)",rusqlite::params![session,track.id,track.title,track.channel]).map_err(|e|e.to_string())?;
                    }
                    crate::settings::notify(&app,&track.title,track.channel.as_deref().unwrap_or("Sela"));
                    if record_history {engine.state.history_revision+=1;}
                    engine.state.recorded=true;
                }
            }
            if let Some(duration)=engine.player.number("duration")?{engine.state.duration=Some(duration);}
            if !engine.state.loading && engine.state.active_id.is_some() { if let (Some(track),Some(duration))=(&engine.state.track,engine.state.duration) { crate::lyrics::refine(&track.id,duration); } }
            crate::scrobbling::observe(&app,engine.state.listening_session,engine.state.track.as_ref(),engine.state.duration,engine.state.position,!engine.state.paused && !engine.state.loading && !engine.state.ended && !engine.player.flag("paused-for-cache")?);
            let _=update_surface(&app,&engine);
            while let Some(event)=engine.player.event()? { match event {
                player::Event::Start(id)=>{if engine.state.active_id!=Some(id){continue}},
                player::Event::Loaded=>{if engine.state.active_id.is_some(){engine.state.loading=false;engine.state.rollback=None;
                    if engine.state.consume_once && engine.state.repeat=="once" {app.state::<Store>().0.lock().map_err(|e|e.to_string())?.execute("UPDATE desktop_playback_options SET repeat='off' WHERE id=1",[]).map_err(|e|e.to_string())?;engine.state.repeat="off".into();}
                    engine.state.consume_once=false;
                }},
                player::Event::End{id,reason,error}=>{
                    if !engine.state.accept_end(id){continue}
                    engine.state.loading=false;
                    let interrupted=reason==0 && premature_end(engine.state.position,engine.state.duration);
                    if reason==0 && !interrupted {engine.state.ended=true;if !engine.state.paused && !engine.state.local{return Ok(Some((engine.state.generation,false,String::new(),String::new(),0.0)))}}
                    else if reason==4 || interrupted {
                        if let Some((mode,source,position))=engine.state.rollback.take(){engine.state.mode=mode;let result=load(&mut engine,&source,position);engine.state.error=Some(format!("Video gagal dimuat ({error}); mode sebelumnya dipulihkan"));result?;continue}
                        engine.state.prepared=None;
                        if engine.state.retry_allowed(){engine.state.failures+=1;engine.state.generation+=1;engine.state.loading=true;return Ok(engine.state.track.as_ref().map(|track|(engine.state.generation,true,track.id.clone(),engine.state.mode.clone(),engine.state.position)))}
                        engine.state.error=Some(format!("Pemutaran gagal ({error}); coba putar lagu lagi"));
                    }
                }
            }}Ok(None)
        })();
        match action {
            Ok(Some((generation,retry,id,mode,position)))=>{
                let app=app.clone();
                tauri::async_runtime::spawn(async move {
                    if retry {let _=finish_load(&app,generation,id,mode,None,position).await;return}
                    for attempt in 0..3 {
                        if attempt>0 {tokio::time::sleep(Duration::from_secs(attempt*2)).await;}
                        {let managed=app.state::<Mutex<Engine>>();let Ok(engine)=managed.lock() else{return};
                            if engine.state.generation!=generation || engine.state.paused{return}
                        }
                        match advance(&app,false,true,Some(generation)).await {
                            Ok(())=>return,
                            Err(error)=>{let managed=app.state::<Mutex<Engine>>();let Ok(mut engine)=managed.lock() else{return};
                                if engine.state.generation!=generation{return}
                                engine.state.error=Some(error);engine.state.loading=false;
                            }
                        }
                    }
                });
            },
            Err(error)=>{if let Ok(mut engine)=app.state::<Mutex<Engine>>().lock(){engine.state.error=Some(error);}},
            _=>{}
        }
        prepare_next(&app);
    }
}
fn prepare_next(app:&tauri::AppHandle){
    let candidate=(||->Result<Option<(u64,String,String)>,String>{
        let managed=lock(app)?;let mut engine=managed.lock().map_err(|e|e.to_string())?;
        if engine.state.loading || engine.state.local || engine.state.track.is_none(){return Ok(None)}
        let store=app.state::<Store>();let db=store.0.lock().map_err(|e|e.to_string())?;let rows=queue::entries(&db)?;
        let next=rows.iter().position(|e|Some(e.entry_id)==engine.state.entry_id).and_then(|i|rows.get(i+1));
        let Some(next)=next else{return Ok(None)};
        let key=(engine.state.generation,next.track.id.clone(),engine.state.mode.clone());
        if engine.state.preparing.as_ref()==Some(&key) || engine.state.prepared.as_ref().is_some_and(|p|p.id==key.1 && p.mode==key.2 && p.at.elapsed()<Duration::from_secs(120)){return Ok(None)}
        engine.state.preparing=Some(key.clone());Ok(Some(key))
    })();
    if let Ok(Some((generation,id,mode)))=candidate {let app=app.clone();tauri::async_runtime::spawn(async move {
        let result=extract(&app,&id,&mode).await;
        if let Ok(mut engine)=app.state::<Mutex<Engine>>().lock(){if engine.state.generation==generation && engine.state.preparing.as_ref()==Some(&(generation,id.clone(),mode.clone())){if let Ok(source)=result{engine.state.preparing=None;engine.state.prepared=Some(Prepared{id,mode,source,at:Instant::now()});}}};
    });}
}
pub(crate) fn system_indonesian()->bool {
    #[cfg(windows)]
    { #[link(name="kernel32")] extern "system" { fn GetUserDefaultUILanguage()->u16; } unsafe { GetUserDefaultUILanguage() & 0x03ff == 0x21 } }
    #[cfg(not(windows))]
    { std::env::var("LANG").unwrap_or_default().starts_with("id") }
}
pub fn hotkey(app:&tauri::AppHandle,action:usize) {
    if action==1 || action==2 {let app=app.clone();tauri::async_runtime::spawn(async move {let result=if action==1 {playback_next(app.clone()).await} else {playback_previous(app.clone()).await};if let Err(error)=result {if let Ok(mut engine)=app.state::<Mutex<Engine>>().lock(){engine.state.error=Some(error);}}});return}
    if let Ok(mut engine)=app.state::<Mutex<Engine>>().lock() {
        let result=if action==0 {let paused=!engine.state.paused;engine.player.command(&["set","pause",if paused {"yes"} else {"no"}]).map(|()|engine.state.paused=paused)}
            else {engine.player.number("volume").and_then(|volume|engine.player.command(&["set","volume",&(volume.unwrap_or(80.0)+if action==3 {5.0} else {-5.0}).clamp(0.0,100.0).to_string()]))};
        if let Err(error)=result {engine.state.error=Some(error);}
    }
}
struct TrayLabels { show: tauri::menu::MenuItem<tauri::Wry>, pause: tauri::menu::MenuItem<tauri::Wry>, exit: tauri::menu::MenuItem<tauri::Wry> }
pub fn set_tray_language(app:&tauri::AppHandle,language:&str)->Result<(),String> {
    let labels=app.state::<TrayLabels>();let english=language=="en" || language=="system" && !system_indonesian();
    labels.show.set_text(if english {"Open Sela"} else {"Buka Sela"}).map_err(|_|"SETTINGS_LANGUAGE")?;
    labels.pause.set_text(if english {"Play / pause"} else {"Putar / jeda"}).map_err(|_|"SETTINGS_LANGUAGE")?;
    labels.exit.set_text(if english {"Quit"} else {"Keluar"}).map_err(|_|"SETTINGS_LANGUAGE")?;
    Ok(())
}
pub fn setup(app:&mut tauri::App)->Result<(),Box<dyn std::error::Error>>{
    let settings=app.state::<crate::settings::SettingsState>().0.lock().map_err(|e|e.to_string())?.clone();
    let mut state=Playback::default();
    state.mode=settings.initial_mode.clone();
    {let store=app.state::<Store>();let db=store.0.lock().map_err(|e|e.to_string())?;
    db.execute_batch("CREATE TABLE IF NOT EXISTS desktop_playback_options(id INTEGER PRIMARY KEY CHECK(id=1),repeat TEXT NOT NULL DEFAULT 'off',autoplay INTEGER NOT NULL DEFAULT 1);INSERT OR IGNORE INTO desktop_playback_options(id) VALUES(1);")?;
    (state.repeat,state.autoplay)=db.query_row("SELECT repeat,autoplay FROM desktop_playback_options WHERE id=1",[],|r|Ok((r.get(0)?,r.get(1)?)))?;}
    #[cfg(windows)] let surface=video_surface::create(app.get_webview_window("main").ok_or("Main window missing")?.hwnd()?.0 as isize)?;
    #[cfg(not(windows))] let surface=0;
    let native=player::Player::with_surface(&app.path().resource_dir()?.join("libmpv-2.dll"),(surface!=0).then_some(surface))?;
    native.command(&["set","volume",&settings.initial_volume.to_string()])?;
    native.command(&["set","vid",if state.mode=="video" {"auto"} else {"no"}])?;
    app.manage(Mutex::new(Engine{player:native,state,surface,bounds:(0.0,0.0,0.0,0.0,false)}));
    app.manage(tokio::sync::Mutex::new(()));
    app.manage(discord::Presence(Mutex::new(None)));
    let english=settings.language=="en" || settings.language=="system" && !system_indonesian();
    let show=tauri::menu::MenuItem::with_id(app,"show",if english {"Open Sela"} else {"Buka Sela"},true,None::<&str>)?;
    let pause=tauri::menu::MenuItem::with_id(app,"pause",if english {"Play / pause"} else {"Putar / jeda"},true,None::<&str>)?;
    let exit=tauri::menu::MenuItem::with_id(app,"exit",if english {"Quit"} else {"Keluar"},true,None::<&str>)?;
    let menu=tauri::menu::Menu::with_items(app,&[&show,&pause,&exit])?;
    app.manage(TrayLabels {show,pause,exit});
    let mut tray=tauri::tray::TrayIconBuilder::new().tooltip("Sela").menu(&menu).on_menu_event(|app,event|match event.id.as_ref(){
        "show"=>{if let Some(window)=app.get_webview_window("main"){let _=window.show();let _=window.set_focus();}},
        "pause"=>{if let Ok(mut engine)=app.state::<Mutex<Engine>>().lock(){let paused=!engine.state.paused;if engine.player.command(&["set","pause",if paused{"yes"}else{"no"}]).is_ok(){engine.state.paused=paused;}}},
        "exit"=>app.exit(0),_=>{}
    });
    if let Some(icon)=app.default_window_icon(){tray=tray.icon(icon.clone());}tray.build(app)?;
    if let Some(window)=app.get_webview_window("main") { let handle=app.handle().clone(); window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. }=event {
            if handle.state::<crate::settings::SettingsState>().0.lock().map(|s|s.close_to_tray).unwrap_or(false) {
                if let Some(window)=handle.get_webview_window("main") { if window.hide().is_ok() {api.prevent_close();} }
            }
        }
    }); }
    tauri::async_runtime::spawn(worker(app.handle().clone()));Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn eof_is_consumed_once_and_stale_events_are_ignored(){let mut state=Playback::default();state.active_id=Some(7);assert!(!state.accept_end(6));assert!(state.accept_end(7));assert!(!state.accept_end(7));}
    #[test] fn pause_blocks_recovery_and_retry_budget_is_bounded(){let mut state=Playback::default();state.paused=true;assert!(!state.retry_allowed());state.paused=false;assert!(state.retry_allowed());state.failures=2;assert!(!state.retry_allowed());}
    #[test] fn previous_restarts_only_above_three_seconds(){assert!(!restart_previous(3.0));assert!(restart_previous(3.01));}
}

#[cfg(test)]
mod stream_tests {
    use super::*;
    #[test] fn early_eof_is_failure_but_short_tail_is_completion(){assert!(premature_end(20.0,Some(100.0)));assert!(!premature_end(99.0,Some(100.0)));assert!(!premature_end(20.0,None));}
}
