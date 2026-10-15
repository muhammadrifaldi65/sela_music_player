use rusqlite::{params, Connection};
use serde::{Deserialize,Serialize};
use serde_json::Value;
use std::{collections::BTreeMap,sync::Mutex,time::{Duration,Instant,SystemTime,UNIX_EPOCH}};
use tauri::Manager;

#[derive(Clone,Default,Serialize,Deserialize)]
struct Secrets { api_key:String, api_secret:String, session:Option<String>, username:Option<String> }
#[derive(Default)]
struct Runtime { secrets:Secrets, token:Option<String>, error:Option<String>, epoch:u64, listen:Option<Listening> }
struct Listening { generation:u64, key:String, started:i64, at:Instant, position:f64, seconds:f64, playing:bool, now_playing:bool, queued:bool }
#[derive(Serialize)]
pub struct Status { configured:bool,connected:bool,username:Option<String>,pending:i64,error:Option<String>,authorizing:bool }

pub fn initialize(db:&Connection)->rusqlite::Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS lastfm_secrets(id INTEGER PRIMARY KEY CHECK(id=1),protected BLOB NOT NULL); CREATE TABLE IF NOT EXISTS lastfm_queue(session TEXT PRIMARY KEY,title TEXT NOT NULL,artist TEXT NOT NULL,started INTEGER NOT NULL,duration REAL NOT NULL,state TEXT NOT NULL DEFAULT 'pending',attempts INTEGER NOT NULL DEFAULT 0,next_attempt INTEGER NOT NULL DEFAULT 0);")
}
fn now()->i64 {SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64}
fn eligible(duration:f64,listened:f64)->bool {duration.is_finite() && duration>30.0 && listened>= (duration/2.0).min(240.0)}
fn listened_delta(elapsed:f64,progress:f64,playing:bool)->f64 {
    if playing && elapsed<=3.0 && progress>0.0 && (progress-elapsed).abs()<=0.75 {elapsed.min(progress)}else{0.0}
}
fn signature(fields:&BTreeMap<String,String>,secret:&str)->String {
    let mut input=String::new();
    for (key,value) in fields {if key!="format" && key!="callback" {input.push_str(key);input.push_str(value);}}
    input.push_str(secret);format!("{:x}",md5::compute(input))
}
#[cfg(target_os="windows")]
fn protect(input:&[u8],encrypt:bool)->Result<Vec<u8>,String> {
    #[repr(C)] struct Blob {len:u32,data:*mut u8}
    #[link(name="crypt32")] extern "system" {
        fn CryptProtectData(input:*const Blob,description:*const u16,entropy:*const Blob,reserved:*mut std::ffi::c_void,prompt:*mut std::ffi::c_void,flags:u32,output:*mut Blob)->i32;
        fn CryptUnprotectData(input:*const Blob,description:*mut *mut u16,entropy:*const Blob,reserved:*mut std::ffi::c_void,prompt:*mut std::ffi::c_void,flags:u32,output:*mut Blob)->i32;
    }
    #[link(name="kernel32")] extern "system" {fn LocalFree(memory:*mut std::ffi::c_void)->*mut std::ffi::c_void;}
    let input=Blob{len:u32::try_from(input.len()).map_err(|_|"lastfm.secure_storage")?,data:input.as_ptr() as *mut u8};
    let mut output=Blob{len:0,data:std::ptr::null_mut()};
    unsafe {
        let ok=if encrypt {CryptProtectData(&input,std::ptr::null(),std::ptr::null(),std::ptr::null_mut(),std::ptr::null_mut(),1,&mut output)}else{CryptUnprotectData(&input,std::ptr::null_mut(),std::ptr::null(),std::ptr::null_mut(),std::ptr::null_mut(),1,&mut output)};
        if ok==0 {return Err("lastfm.secure_storage".into())}
        let result=std::slice::from_raw_parts(output.data,output.len as usize).to_vec();LocalFree(output.data.cast());Ok(result)
    }
}
#[cfg(not(target_os="windows"))]
fn protect(_: &[u8],_:bool)->Result<Vec<u8>,String> {Err("lastfm.unsupported_platform".into())}
fn same_account(previous:&Secrets,next:&Secrets)->bool {
    matches!((&previous.username,&next.username),(Some(a),Some(b)) if a.eq_ignore_ascii_case(b))
}
fn persist_record(db:&mut Connection,bytes:Vec<u8>,clear:bool)->Result<(),String> {
    let tx=db.transaction().map_err(|_|"data.database")?;
    tx.execute("INSERT INTO lastfm_secrets(id,protected) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET protected=excluded.protected",[bytes]).map_err(|_|"data.database")?;
    if clear {tx.execute("DELETE FROM lastfm_queue",[]).map_err(|_|"data.database")?;}
    tx.commit().map_err(|_|"data.database".into())
}
fn persist(app:&tauri::AppHandle,secrets:&Secrets,clear:bool)->Result<(),String> {
    let bytes=protect(&serde_json::to_vec(secrets).map_err(|_|"lastfm.secure_storage")?,true)?;
    let store=app.state::<crate::Store>();let mut db=store.0.lock().map_err(|_|"data.database")?;
    persist_record(&mut db,bytes,clear)
}
pub fn setup(app:&tauri::AppHandle)->Result<(),String> {
    use rusqlite::OptionalExtension;
    let blob:Option<Vec<u8>>=app.state::<crate::Store>().0.lock().map_err(|_|"data.database")?.query_row("SELECT protected FROM lastfm_secrets WHERE id=1",[],|r|r.get(0)).optional().map_err(|_|"data.database")?;
    let mut runtime=Runtime::default();
    if let Some(blob)=blob {match protect(&blob,false).and_then(|bytes|serde_json::from_slice(&bytes).map_err(|_|"lastfm.secure_storage".into())) {Ok(secrets)=>runtime.secrets=secrets,Err(error)=>runtime.error=Some(error)}}
    app.manage(Mutex::new(runtime));
    let app=app.clone();tauri::async_runtime::spawn(async move {loop {tokio::time::sleep(Duration::from_secs(5)).await;send_pending(&app).await;}});Ok(())
}
#[tauri::command]
pub fn lastfm_state(app:tauri::AppHandle)->Result<Status,String> {
    let managed=app.state::<Mutex<Runtime>>();let rt=managed.lock().map_err(|_|"lastfm.internal")?;
    let pending=app.state::<crate::Store>().0.lock().map_err(|_|"data.database")?.query_row("SELECT count(*) FROM lastfm_queue WHERE state='pending'",[],|r|r.get(0)).map_err(|_|"data.database")?;
    Ok(Status{configured:!rt.secrets.api_key.is_empty(),connected:rt.secrets.session.is_some(),username:rt.secrets.username.clone(),pending,error:rt.error.clone(),authorizing:rt.token.is_some()})
}
#[tauri::command]
pub fn lastfm_configure(api_key:String,api_secret:String,app:tauri::AppHandle)->Result<Status,String> {
    if [&api_key,&api_secret].iter().any(|s|s.len()!=32 || !s.bytes().all(|b|b.is_ascii_hexdigit())) {return Err("lastfm.invalid_credentials".into())}
    {let managed=app.state::<Mutex<Runtime>>();let mut rt=managed.lock().map_err(|_|"lastfm.internal")?;
    let secrets=Secrets{api_key,api_secret,..Secrets::default()};persist(&app,&secrets,true)?;rt.secrets=secrets;rt.epoch+=1;rt.token=None;rt.listen=None;rt.error=None;}
    lastfm_state(app)
}
#[derive(Debug)]
struct ApiError {code:i64,message:String}
async fn request(secrets:&Secrets,method:&str,fields:impl IntoIterator<Item=(&'static str,String)>)->Result<Value,ApiError> {
    let mut fields:BTreeMap<String,String>=fields.into_iter().map(|(k,v)|(k.into(),v)).collect();
    fields.insert("method".into(),method.into());fields.insert("api_key".into(),secrets.api_key.clone());
    fields.insert("api_sig".into(),signature(&fields,&secrets.api_secret));fields.insert("format".into(),"json".into());
    let fail=||ApiError{code:-1,message:"lastfm.network".into()};
    let client=reqwest::Client::builder().timeout(Duration::from_secs(20)).build().map_err(|_|fail())?;
    let request=if method.starts_with("auth.") {client.get("https://ws.audioscrobbler.com/2.0/").query(&fields)}else{client.post("https://ws.audioscrobbler.com/2.0/").form(&fields)};
    let response=request.send().await.map_err(|_|fail())?;
    let status=response.status();let body:Value=response.json().await.map_err(|_|fail())?;
    if let Some(code)=body["error"].as_i64() {return Err(ApiError{code,message:format!("lastfm.api.{code}")})}
    if !status.is_success() {return Err(fail())}Ok(body)
}
#[tauri::command]
pub async fn lastfm_begin_auth(app:tauri::AppHandle)->Result<Value,String> {
    let (secrets,epoch)={let managed=app.state::<Mutex<Runtime>>();let rt=managed.lock().map_err(|_|"lastfm.internal")?;if rt.secrets.api_key.is_empty(){return Err("lastfm.credentials_required".into())}(rt.secrets.clone(),rt.epoch)};
    let body=request(&secrets,"auth.getToken",[]).await.map_err(|e|e.message)?;
    let token=body["token"].as_str().filter(|s|s.len()==32 && s.bytes().all(|b|b.is_ascii_hexdigit())).ok_or("lastfm.invalid_response")?.to_owned();
    let url=format!("https://www.last.fm/api/auth/?api_key={}&token={}",secrets.api_key,token);
    {let managed=app.state::<Mutex<Runtime>>();let mut rt=managed.lock().map_err(|_|"lastfm.internal")?;if rt.epoch!=epoch{return Err("lastfm.cancelled".into())}rt.token=Some(token);}
    #[cfg(target_os="windows")]
    {std::process::Command::new("rundll32.exe").args(["url.dll,FileProtocolHandler",&url]).spawn().map_err(|_|"lastfm.browser_failed")?;}
    #[cfg(not(target_os="windows"))]
    return Err("lastfm.unsupported_platform".into());
    #[cfg(target_os="windows")]
    Ok(serde_json::json!({"url":url}))
}
#[tauri::command]
pub async fn lastfm_complete_auth(app:tauri::AppHandle)->Result<Status,String> {
    let (mut secrets,token,epoch)={let managed=app.state::<Mutex<Runtime>>();let rt=managed.lock().map_err(|_|"lastfm.internal")?;(rt.secrets.clone(),rt.token.clone().ok_or("lastfm.authorization_required")?,rt.epoch)};
    let body=request(&secrets,"auth.getSession",[("token",token)]).await.map_err(|e|e.message)?;
    secrets.session=Some(body["session"]["key"].as_str().filter(|s|!s.is_empty()&&s.len()<=128).ok_or("lastfm.invalid_response")?.into());
    secrets.username=Some(body["session"]["name"].as_str().filter(|s|!s.is_empty()&&s.len()<=256).ok_or("lastfm.invalid_response")?.into());
    {let managed=app.state::<Mutex<Runtime>>();let mut rt=managed.lock().map_err(|_|"lastfm.internal")?;if rt.epoch!=epoch{return Err("lastfm.cancelled".into())}persist(&app,&secrets,!same_account(&rt.secrets,&secrets))?;rt.secrets=secrets;rt.token=None;rt.error=None;rt.epoch+=1;rt.listen=None;}
    lastfm_state(app)
}
#[tauri::command]
pub fn lastfm_disconnect(app:tauri::AppHandle)->Result<Status,String> {
    {let managed=app.state::<Mutex<Runtime>>();let mut rt=managed.lock().map_err(|_|"lastfm.internal")?;let mut secrets=rt.secrets.clone();secrets.session=None;secrets.username=None;persist(&app,&secrets,true)?;rt.secrets=secrets;rt.epoch+=1;rt.token=None;rt.listen=None;rt.error=None;}
    lastfm_state(app)
}
fn enqueue(db:&Connection,key:&str,title:&str,artist:&str,started:i64,duration:f64)->Result<(),String> {
    db.execute("INSERT OR IGNORE INTO lastfm_queue(session,title,artist,started,duration) VALUES(?1,?2,?3,?4,?5)",params![key,title,artist,started,duration]).map_err(|_|"data.database")?;Ok(())
}
pub fn observe(app:&tauri::AppHandle,generation:u64,track:Option<&crate::library::Track>,duration:Option<f64>,position:f64,playing:bool) {
    let Some(track)=track else{return};let Some(artist)=track.channel.as_deref().filter(|s|!s.trim().is_empty()) else{return};
    if !position.is_finite(){return}
    let managed=app.state::<Mutex<Runtime>>();let Ok(mut rt)=managed.lock() else{return};if rt.secrets.session.is_none(){return}
    let instant=Instant::now();
    if rt.listen.as_ref().is_none_or(|l|l.generation!=generation) {
        let stamp=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        rt.listen=Some(Listening{generation,key:format!("{stamp}-{generation}"),started:now(),at:instant,position,seconds:0.0,playing,now_playing:false,queued:false});
    }
    let listen=rt.listen.as_mut().unwrap();
    listen.seconds+=listened_delta(instant.duration_since(listen.at).as_secs_f64(),position-listen.position,playing&&listen.playing);
    listen.at=instant;listen.position=position;listen.playing=playing;
    if let Some(duration)=duration.filter(|d|d.is_finite()&&*d>0.0) {
        if !listen.queued && eligible(duration,listen.seconds) {
            let result=app.state::<crate::Store>().0.lock().map_err(|_|"data.database".to_owned()).and_then(|db|enqueue(&db,&listen.key,&track.title,artist,listen.started,duration));
            if result.is_ok(){listen.queued=true}else{rt.error=result.err();return}
        }
    }
    let listen=rt.listen.as_mut().unwrap();
    if playing && !listen.now_playing {
        listen.now_playing=true;
        let secrets=rt.secrets.clone();let epoch=rt.epoch;let app=app.clone();let title=track.title.clone();let artist=artist.to_owned();
        tauri::async_runtime::spawn(async move {
            {let managed=app.state::<Mutex<Runtime>>();let Ok(rt)=managed.lock() else{return};if rt.epoch!=epoch{return}}
            let result=request(&secrets,"track.updateNowPlaying",[("sk",secrets.session.clone().unwrap()),("track",title),("artist",artist)]).await;
            let failure=match result {Ok(body) if accepted(&body["nowplaying"])=>None,Ok(_)=>Some(ApiError{code:0,message:"lastfm.scrobble_ignored".into()}),Err(error)=>Some(error)};
            if let Some(error)=failure {
                let managed=app.state::<Mutex<Runtime>>();if let Ok(mut rt)=managed.lock(){if rt.epoch==epoch {
                    if error.code==9 {
                        let mut secrets=rt.secrets.clone();secrets.session=None;
                        if let Err(error)=persist(&app,&secrets,false){rt.error=Some(error);return}
                        rt.secrets=secrets;rt.epoch+=1;rt.listen=None;
                    }
                    rt.error=Some(error.message);
                }};
            }
        });
    }
}
fn accepted(item:&Value)->bool {let code=&item["ignoredMessage"]["code"];code.as_str()==Some("0")||code.as_i64()==Some(0)}
async fn send_pending(app:&tauri::AppHandle) {
    let (secrets,epoch)={let managed=app.state::<Mutex<Runtime>>();let Ok(rt)=managed.lock() else{return};if rt.secrets.session.is_none(){return}(rt.secrets.clone(),rt.epoch)};
    let row={let store=app.state::<crate::Store>();let Ok(db)=store.0.lock() else{return};db.query_row("SELECT session,title,artist,started,duration,attempts,next_attempt FROM lastfm_queue WHERE state='pending' ORDER BY started,rowid LIMIT 1",[],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,i64>(3)?,r.get::<_,f64>(4)?,r.get::<_,u32>(5)?,r.get::<_,i64>(6)?))).ok()};
    let Some((key,title,artist,started,duration,attempts,next))=row else{return};if next>now(){return}
    {let managed=app.state::<Mutex<Runtime>>();let Ok(rt)=managed.lock() else{return};if rt.epoch!=epoch{return}}
    let result=request(&secrets,"track.scrobble",[("sk",secrets.session.clone().unwrap()),("track",title),("artist",artist),("timestamp",started.to_string()),("duration",(duration as u64).to_string())]).await;
    let managed=app.state::<Mutex<Runtime>>();let Ok(mut rt)=managed.lock() else{return};if rt.epoch!=epoch{return}
    let (state,error)=match result {
        Ok(body)=> {
            if accepted(&body["scrobbles"]["scrobble"]) {("sent",None)}else{("failed",Some("lastfm.scrobble_ignored".into()))}
        },
        Err(error) if error.code==9=> {
            let mut secrets=rt.secrets.clone();secrets.session=None;
            if let Err(error)=persist(app,&secrets,false){rt.error=Some(error);return}
            rt.secrets=secrets;rt.epoch+=1;rt.listen=None;rt.error=Some("lastfm.api.9".into());return
        },
        Err(error)=> (if matches!(error.code,-1|11|16){"pending"}else{"failed"},Some(error.message)),
    };
    let delay=30_i64.saturating_mul(1_i64<<attempts.min(10)).min(3600);
    let store=app.state::<crate::Store>();let Ok(db)=store.0.lock() else{return};
    if db.execute("UPDATE lastfm_queue SET state=?1,attempts=attempts+1,next_attempt=?2 WHERE session=?3",params![state,now()+delay,key]).is_err(){rt.error=Some("data.database".into());return}
    rt.error=error;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn reauthorization_preserves_same_account_queue_and_clears_other_accounts() {
        let mut db=Connection::open_in_memory().unwrap();initialize(&db).unwrap();
        enqueue(&db,"listen","Song","Artist",100,180.0).unwrap();
        let mut previous=Secrets{username:Some("listener".into()),session:Some("expired".into()),..Secrets::default()};
        previous.session=None;
        persist_record(&mut db,vec![1],false).unwrap();
        let mut next=Secrets{username:Some("Listener".into()),session:Some("renewed".into()),..Secrets::default()};
        persist_record(&mut db,vec![2],!same_account(&previous,&next)).unwrap();
        assert_eq!(db.query_row("SELECT count(*) FROM lastfm_queue WHERE state='pending'",[],|r|r.get::<_,i64>(0)).unwrap(),1);
        next.username=Some("other-account".into());
        persist_record(&mut db,vec![3],!same_account(&previous,&next)).unwrap();
        assert_eq!(db.query_row("SELECT count(*) FROM lastfm_queue",[],|r|r.get::<_,i64>(0)).unwrap(),0);
        enqueue(&db,"new-listen","Song","Artist",101,180.0).unwrap();
        persist_record(&mut db,vec![4],true).unwrap();
        assert_eq!(db.query_row("SELECT count(*) FROM lastfm_queue",[],|r|r.get::<_,i64>(0)).unwrap(),0);
    }
    #[test] fn ignored_or_missing_response_is_not_success() {
        assert!(accepted(&serde_json::json!({"ignoredMessage":{"code":"0"}})));
        assert!(!accepted(&serde_json::json!({"ignoredMessage":{"code":"3"}})));
        assert!(!accepted(&Value::Null));
    }
    #[cfg(target_os="windows")]
    #[test] fn protected_secrets_roundtrip_without_plaintext() {
        let plain=b"private-lastfm-session-key";let encrypted=protect(plain,true).unwrap();
        assert!(!encrypted.windows(plain.len()).any(|bytes|bytes==plain));
        assert_eq!(protect(&encrypted,false).unwrap(),plain);
    }
    #[test] fn eligibility_requires_real_time_and_long_track() {
        assert!(!eligible(30.0,240.0)); assert!(!eligible(180.0,89.9));
        assert!(eligible(180.0,90.0)); assert!(eligible(900.0,240.0));
        assert_eq!(listened_delta(1.0,100.0,true),0.0);
        assert_eq!(listened_delta(1.0,1.0,false),0.0);
        assert_eq!(listened_delta(1.0,1.0,true),1.0);
        assert_eq!(listened_delta(30.0,30.0,true),0.0);
    }
    #[test] fn signing_sorts_keys_and_excludes_format() {
        let fields=BTreeMap::from([("b".into(),"2".into()),("a".into(),"1".into()),("format".into(),"json".into())]);
        assert_eq!(signature(&fields,"secret"),format!("{:x}",md5::compute("a1b2secret")));
    }
    #[test] fn queue_deduplicates_retries_not_replays() {
        let db=Connection::open_in_memory().unwrap(); initialize(&db).unwrap();
        for id in ["one","one","two"] {enqueue(&db,id,"Song","Artist",100,180.0).unwrap();}
        assert_eq!(db.query_row("SELECT count(*) FROM lastfm_queue",[],|r|r.get::<_,i64>(0)).unwrap(),2);
    }
}
