import {useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {playback,platform} from './playback';
export function Discord(){
 const [enabled,setEnabled]=useState(()=>localStorage.getItem('discord-enabled')!=='false'),[status,setStatus]=useState('Menunggu pemutaran'),[desktop,setDesktop]=useState(false);
 useEffect(()=>{localStorage.removeItem('discord-application-id');void platform.then(value=>setDesktop(value==='windows')).catch(e=>setStatus(String(e)))},[]);
 useEffect(()=>{
  if(!desktop)return;
  let alive=true,busy=false,connected=false,nextAttempt=0;
  setStatus(enabled?'Menunggu pemutaran':'Nonaktif');
  if(!enabled){void invoke('discord_clear').catch(e=>{if(alive)setStatus(String(e))});return()=>{alive=false}}
  const update=async()=>{
   if(busy||playback.loading)return;
   busy=true;const track=playback.current?.track;
   try{
    const state=await invoke<{position:number|null;duration:number|null;paused:boolean;ended:boolean}>('playback_state');
    if(!alive)return;
    if(!track||state.position===null||!state.duration||state.ended){if(connected)await invoke('discord_clear');connected=false;nextAttempt=0;setStatus('Menunggu pemutaran');return}
    if(!connected){if(Date.now()<nextAttempt)return;nextAttempt=Date.now()+30000;await invoke('discord_connect');connected=true}
    if(!alive){await invoke('discord_clear');return}
    await invoke('discord_update',{title:track.title,artist:track.channel??'Artis tidak diketahui',videoId:track.id,position:state.position,duration:state.duration,paused:state.paused});
    if(alive)setStatus(state.paused?'Terhubung · dijeda':'Terhubung · Listening');else await invoke('discord_clear');
   }catch(e){if(alive)setStatus(String(e));connected=false;await invoke('discord_clear').catch(()=>{})}finally{busy=false}
  };
  void update();const timer=setInterval(update,5000);
  return()=>{alive=false;clearInterval(timer);void invoke('discord_clear').catch(()=>{})};
 },[enabled,desktop]);
 if(!desktop)return null;
 return <div className="discord-control"><span className="discord-status" role="status">{status}</span><button className="discord-toggle" aria-label="Discord Rich Presence" aria-pressed={enabled} title={`${enabled?'Nonaktifkan':'Aktifkan'} Discord Rich Presence — ${status}`} onClick={()=>{setEnabled(!enabled);localStorage.setItem('discord-enabled',String(!enabled))}}><svg viewBox="0 0 24 24" aria-hidden="true"><path fill="currentColor" d="M19.7 5.1a18 18 0 0 0-4.4-1.4l-.6 1.2a16 16 0 0 0-5.4 0l-.6-1.2a18 18 0 0 0-4.4 1.4C1.5 9.3.8 13.4 1.2 17.5a18 18 0 0 0 5.4 2.8l1.1-1.8-1.7-.8.4-.3a13 13 0 0 0 11.2 0l.4.3-1.7.8 1.1 1.8a18 18 0 0 0 5.4-2.8c.5-4.8-.8-8.8-3.1-12.4ZM8.5 15.1c-1 0-1.8-.9-1.8-2s.8-2 1.8-2 1.8.9 1.8 2-.8 2-1.8 2Zm7 0c-1 0-1.8-.9-1.8-2s.8-2 1.8-2 1.8.9 1.8 2-.8 2-1.8 2Z"/></svg><span>{enabled?'On':'Off'}</span></button></div>;
}
