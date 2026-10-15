import {useTranslation,t as tr,errorText} from './i18n';
import {useEffect,useRef,useState,type CSSProperties} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {playback,platform,changePlayback} from './playback';
import {Lyrics} from './Lyrics';
export function NowPlaying(){
 const {t:tr,locale}=useTranslation();
 const [open,setOpen]=useState(false),[,refresh]=useState(0),[desktop,setDesktop]=useState(false),[switching,setSwitching]=useState(false),[error,setError]=useState('');
 const back=useRef<HTMLButtonElement>(null),surface=useRef<HTMLDivElement>(null),panel=useRef<HTMLElement>(null);
 const surfaceWrites=useRef(Promise.resolve());
 const track=playback.selected??playback.current?.track;
 useEffect(()=>{if(!open)return;const previous=document.activeElement as HTMLElement|null;back.current?.focus();const key=(event:KeyboardEvent)=>{if(event.key==='Escape'&&!document.querySelector('dialog[open]')){event.preventDefault();setOpen(false)}};window.addEventListener('keydown',key);return()=>{window.removeEventListener('keydown',key);previous?.focus()}},[open]);
 useEffect(()=>{void platform.then(value=>setDesktop(value!=='android')).catch(e=>setError(errorText(e)));const show=()=>{if(playback.current||playback.selected)setOpen(v=>!v)};const changed=()=>refresh(v=>v+1);window.addEventListener('toggle-now-playing',show);window.addEventListener('playback-changed',changed);return()=>{window.removeEventListener('toggle-now-playing',show);window.removeEventListener('playback-changed',changed)}},[]);
 useEffect(()=>{
  if(!desktop||!open||!surface.current)return;
  let frame=0,alive=true,busy=false;
  type Bounds={x:number;y:number;width:number;height:number;visible:boolean};
  let pending:Bounds|null=null;
  const send=async()=>{if(busy)return;busy=true;try{while(pending){const bounds=pending;pending=null;surfaceWrites.current=surfaceWrites.current.then(async()=>{try{await invoke('set_video_bounds',bounds)}catch(e){if(alive)setError(errorText(e))}});await surfaceWrites.current}}finally{busy=false}};
  const measure=()=>{frame=0;const rect=surface.current?.getBoundingClientRect(),clip=panel.current?.getBoundingClientRect();if(!rect||!clip)return;
   const fullyVisible=rect.top>=clip.top&&rect.bottom<=clip.bottom&&rect.left>=0&&rect.right<=window.innerWidth;
   const obscured=!!document.querySelector('dialog[open],.queue-container.open');
   pending={x:rect.x,y:rect.y,width:rect.width,height:rect.height,visible:playback.mode==='video'&&!document.hidden&&fullyVisible&&!obscured};void send();
  };
  const schedule=()=>{if(!frame)frame=requestAnimationFrame(measure)};
  const observer=new ResizeObserver(schedule);observer.observe(surface.current);if(panel.current)observer.observe(panel.current);
  const overlays=new MutationObserver(schedule);overlays.observe(document.body,{subtree:true,attributes:true,attributeFilter:['class','open'],childList:true});
  window.addEventListener('playback-changed',schedule);window.addEventListener('resize',schedule);window.addEventListener('scroll',schedule,true);document.addEventListener('visibilitychange',schedule);window.visualViewport?.addEventListener('resize',schedule);window.visualViewport?.addEventListener('scroll',schedule);schedule();
  return()=>{alive=false;cancelAnimationFrame(frame);observer.disconnect();overlays.disconnect();window.removeEventListener('playback-changed',schedule);window.removeEventListener('resize',schedule);window.removeEventListener('scroll',schedule,true);document.removeEventListener('visibilitychange',schedule);window.visualViewport?.removeEventListener('resize',schedule);window.visualViewport?.removeEventListener('scroll',schedule);pending=null;surfaceWrites.current=surfaceWrites.current.then(async()=>{try{await invoke('set_video_bounds',{x:0,y:0,width:0,height:0,visible:false})}catch(e){setError(errorText(e))}})};
 },[desktop,open,!!track]);
 if(!open||!track)return null;
 const artwork=`https://i.ytimg.com/vi/${track.id}/hqdefault.jpg`;
 return <section ref={panel} className="expanded-player" style={{'--artwork':`url("${artwork}")`} as CSSProperties} aria-label={tr("Sedang diputar")}><button ref={back} className="back-player" onClick={()=>setOpen(false)}>{tr("Kembali")}</button><div className="expanded-columns"><div className="expanded-art">
  {desktop&&<div className="media-mode" role="group" aria-label={tr("Mode pemutaran")}>{(['audio','video'] as const).map(mode=><button key={mode} aria-pressed={playback.mode===mode} disabled={switching||playback.loading} onClick={async()=>{setSwitching(true);setError('');try{await changePlayback('set_video_mode',mode)}catch(e){setError(errorText(e))}finally{setSwitching(false)}}}>{mode==='audio'?tr("Audio"):tr("Video")}</button>)}</div>}
  <div ref={surface} className={`media-surface ${playback.mode==='video'?'video-mode':''}`} aria-label={playback.mode==='video'?tr("Video lagu"):tr("Artwork lagu")}><img src={artwork} alt={tr('Artwork {title}',{title:track.title})}/></div>
  {(switching||playback.loading)&&<p role="status" className="media-status">{tr("Menyiapkan media…")}</p>}{error&&<p role="alert">{error}</p>}<h2>{track.title}</h2><p>{track.channel}</p></div><Lyrics/></div></section>;
}
