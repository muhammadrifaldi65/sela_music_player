use serde_json::Value;
static NEXT_REQUEST: tokio::sync::Mutex<Option<std::time::Instant>> = tokio::sync::Mutex::const_new(None);
#[tauri::command]
pub async fn lyrics(title:String,artist:String,duration:f64)->Result<Value,String>{
 if title.trim().is_empty()||artist.trim().is_empty()||title.len()>500||artist.len()>500||!duration.is_finite()||!(1.0..=3600.0).contains(&duration){return Err("Metadata belum cukup untuk mencari lirik".into())}
 let mut next=NEXT_REQUEST.lock().await;
 if let Some(deadline)=*next {if deadline>std::time::Instant::now(){return Err("Penyedia lirik meminta jeda; coba lagu ini lagi nanti".into())}}
 *next=Some(std::time::Instant::now()+std::time::Duration::from_millis(500));
 let client=reqwest::Client::builder().timeout(std::time::Duration::from_secs(20)).user_agent("Sela/0.1.0 (By PGM Community)").build().map_err(|e|e.to_string())?;
 let response=client.get("https://lrclib.net/api/get").query(&[("track_name",title),("artist_name",artist),("duration",duration.to_string())]).send().await.map_err(|_|"Penyedia lirik tidak dapat dijangkau")?;
 if matches!(response.status().as_u16(),429|503){
  let seconds=response.headers().get("retry-after").and_then(|v|v.to_str().ok()).and_then(|v|v.parse::<u64>().ok()).unwrap_or(60);
  *next=std::time::Instant::now().checked_add(std::time::Duration::from_secs(seconds.max(1)));
  return Err(format!("Penyedia lirik meminta jeda {seconds} detik"));
 }
 if response.status().as_u16()==404{return Err("Lirik tidak tersedia untuk lagu ini".into())}
 if !response.status().is_success(){return Err("Penyedia lirik sedang tidak tersedia; coba lagi nanti".into())}
 response.json().await.map_err(|_|"Respons lirik tidak valid".into())
}
