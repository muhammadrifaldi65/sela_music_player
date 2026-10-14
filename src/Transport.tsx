import {useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {playback,playTrack,platform,type Entry,type Track} from './playback';
import {Icon} from './Icon';
import {advance,type RepeatMode} from './advance';
function clock(seconds:number|null){if(seconds===null)return '0:00';return `${Math.floor(seconds/60)}:${Math.floor(seconds%60).toString().padStart(2,'0')}`}
export function Transport(){
 const [state,setState]=useState({position:null as number|null,duration:null as number|null,volume:100,paused:true,ended:false});
 const [entries,setEntries]=useState<Entry[]>([]),[error,setError]=useState(''),[,render]=useState(0);
 const [repeat,setRepeat]=useState<RepeatMode>(()=>(localStorage.getItem('repeat-mode')??'off') as RepeatMode),[autoplay,setAutoplay]=useState(()=>localStorage.getItem('autoplay')!=='false');
 const [mobile,setMobile]=useState(false),[nativeReady,setNativeReady]=useState(false);
 useEffect(()=>{void platform.then(value=>setMobile(value==='android')).catch(e=>setError(String(e)))},[]);
 useEffect(()=>{if(mobile&&nativeReady)void invoke('set_playback_options',{repeat,autoplay}).catch(e=>setError(String(e)))},[mobile,nativeReady,repeat,autoplay]);
 useEffect(()=>{let active=true,pending=false,handledEnd:number|null=null;
 const changed=()=>{render(n=>n+1);void invoke<Entry[]>('queue_entries').then(rows=>{if(active)setEntries(rows)}).catch(e=>{if(active)setError(String(e))})};
 changed();window.addEventListener('playback-changed',changed);window.addEventListener('queue-changed',changed);
 void invoke<Entry|null>('current_queue_entry').then(entry=>{if(active&&!playback.current&&!playback.loading){playback.current=entry;render(n=>n+1)}}).catch(e=>{if(active)setError(String(e))});
 const timer=setInterval(async()=>{if(pending)return;pending=true;try{
  const next=await invoke<typeof state & {loading?:boolean;error?:string;track?:Track|null;entryId?:number|null;repeat?:RepeatMode;autoplay?:boolean}>('playback_state');if(active)setState(next);
  if(await platform==='android'){
   if(!active)return;
   if(next.repeat)setRepeat(next.repeat);if(next.autoplay!==undefined)setAutoplay(next.autoplay);setNativeReady(true);
   if(next.error)setError(next.error);
   const changedEntry=(playback.current?.entry_id??null)!==(next.entryId??null);
   playback.current=next.track&&next.entryId!=null?{entry_id:next.entryId,track:next.track}:null;
   if(changedEntry){window.dispatchEvent(new Event('playback-changed'));window.dispatchEvent(new Event('history-changed'))}
   return;
  }
  if(!next.ended)handledEnd=null;
  const current=playback.current;
  if(current&&!playback.loading&&playback.session&&!playback.recorded&&(next.position??0)>0){const session=playback.session;await invoke('record_listen',{session,entryId:current.entry_id});if(playback.session===session)playback.recorded=true;window.dispatchEvent(new Event('history-changed'))}
  if(next.ended&&current&&!playback.loading&&handledEnd!==current.entry_id){
   handledEnd=current.entry_id;
   const rows=await invoke<Entry[]>('queue_entries');
   if(playback.loading||playback.current?.entry_id!==current.entry_id)return;
   const index=rows.findIndex(row=>row.entry_id===current.entry_id);
   const mode=(localStorage.getItem('repeat-mode')??'off') as RepeatMode;
   const decision=advance(index,rows.length,mode,localStorage.getItem('autoplay')!=='false');
   if(decision.repeat!==mode){localStorage.setItem('repeat-mode',decision.repeat);setRepeat(decision.repeat)}
   if(active&&decision.index!==null){const target=rows[decision.index];await playTrack(target.track,target.entry_id)}
   else if(active&&decision.recommend){const result=await invoke<{entries?:{id:string;title:string;channel?:string}[]}>('search',{query:`https://music.youtube.com/watch?v=${current.track.id}&list=RDAMVM${current.track.id}`});const target=result.entries?.find(t=>!rows.some(r=>r.track.id===t.id));if(target)await playTrack(target);else setError('Rekomendasi lanjutan tidak tersedia')}
  }
 }catch(e){if(active)setError(String(e))}finally{pending=false}},500);
 return()=>{active=false;clearInterval(timer);window.removeEventListener('playback-changed',changed);window.removeEventListener('queue-changed',changed)}},[]);
 const index=entries.findIndex(e=>e.entry_id===playback.current?.entry_id);
 async function skip(direction:number){try{if(await platform==='android'){await invoke(direction<0?'playback_previous':'playback_next');return}if(direction<0&&(state.position??0)>3){await invoke('seek',{seconds:0});return}const entry=entries[index+direction];if(entry)await playTrack(entry.track,entry.entry_id)}catch(e){setError(String(e))}}
 const current=playback.current?.track;
 const repeatLabels:Record<RepeatMode,string>={off:'Ulang mati',once:'Ulang sekali',song:'Ulang lagu terus',queue:'Ulang antrean'};
 return <footer className="player-bar" aria-label="Kontrol pemutar">
 <div className="playback-options"><label><input type="checkbox" checked={autoplay} onChange={e=>{setAutoplay(e.target.checked);localStorage.setItem('autoplay',String(e.target.checked))}}/> Autoplay</label><button className="repeat-button" aria-label={repeatLabels[repeat]} title={`${repeatLabels[repeat]} — klik untuk ganti`} aria-pressed={repeat!=='off'} onClick={()=>{const modes:RepeatMode[]=['off','once','song','queue'];const next=modes[(modes.indexOf(repeat)+1)%modes.length];setRepeat(next);localStorage.setItem('repeat-mode',next)}}><svg width="23" height="23" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" aria-hidden="true"><path d="M4 10V8a3 3 0 0 1 3-3h13m-4-3 4 3-4 3M20 14v2a3 3 0 0 1-3 3H4m4-3-4 3 4 3"/></svg><span>{repeat==='once'?'1×':repeat==='song'?'∞':repeat==='queue'?'≡':''}</span></button></div>
 <button className="now-playing" aria-label="Buka artwork dan lirik" disabled={!current} onClick={()=>window.dispatchEvent(new Event('toggle-now-playing'))}>{current?<img src={`https://i.ytimg.com/vi/${current.id}/mqdefault.jpg`} alt=""/>:<div className="empty-art"><Icon name="volume"/></div>}<div><strong>{current?.title??'Temukan lagu berikutnya'}</strong><span>{playback.loading?'Menyiapkan audio…':current?.channel??'Sela · By PGM Community'}</span></div></button>
 <div className="transport"><div className="transport-buttons"><button className="icon-button" aria-label="Lagu sebelumnya" title="Lagu sebelumnya" disabled={playback.loading||(index<=0&&(state.position??0)<=3)} onClick={()=>void skip(-1)}><Icon name="previous"/></button><button className="play-button" aria-label={state.paused||state.position===null?'Putar':'Jeda'} title="Putar / jeda" disabled={playback.loading||(!current&&state.position===null)} onClick={async()=>{try{if(state.position===null&&playback.current)await playTrack(playback.current.track,playback.current.entry_id);else await invoke('set_paused',{paused:!state.paused})}catch(e){setError(String(e))}}}><Icon name={state.paused||state.position===null?'play':'pause'}/></button><button className="icon-button" aria-label="Lagu berikutnya" title="Lagu berikutnya" disabled={playback.loading||index<0||index>=entries.length-1} onClick={()=>void skip(1)}><Icon name="next"/></button></div>
 <div className="timeline"><span>{clock(state.position)}</span><input aria-label="Posisi lagu" type="range" min="0" max={state.duration??0} step="1" value={state.position??0} disabled={!state.duration} onChange={e=>void invoke('seek',{seconds:Number(e.target.value)}).catch(e=>setError(String(e)))}/><span>{clock(state.duration)}</span></div></div>
 <div className="player-extras"><Icon name="volume"/><input aria-label="Volume" type="range" min="0" max="100" value={state.volume} onChange={e=>void invoke('set_volume',{volume:Number(e.target.value)}).catch(e=>setError(String(e)))}/><button className="icon-button" aria-label="Lihat antrean" title="Lihat antrean" onClick={()=>window.dispatchEvent(new Event('toggle-queue'))}><Icon name="queue"/></button></div>{error&&<p role="alert" className="player-error" onClick={()=>setError('')}>{error}</p>}
 </footer>
}
