import {useEffect,useRef,useState} from 'react';
import {playback} from './playback';
import {Lyrics} from './Lyrics';
export function NowPlaying(){
 const [open,setOpen]=useState(false),[,refresh]=useState(0);
 const back=useRef<HTMLButtonElement>(null);
 useEffect(()=>{const show=()=>{if(playback.current)setOpen(v=>!v)};const changed=()=>refresh(v=>v+1);window.addEventListener('toggle-now-playing',show);window.addEventListener('playback-changed',changed);return()=>{window.removeEventListener('toggle-now-playing',show);window.removeEventListener('playback-changed',changed)}},[]);
 useEffect(()=>{if(!open)return;const previous=document.activeElement as HTMLElement|null;back.current?.focus();const key=(event:KeyboardEvent)=>{if(event.key==='Escape'){event.preventDefault();setOpen(false)}};window.addEventListener('keydown',key);return()=>{window.removeEventListener('keydown',key);previous?.focus()}},[open]);
 const track=playback.current?.track;
 if(!open||!track)return null;
 return <section className="expanded-player" aria-label="Sedang diputar"><button ref={back} className="back-player" onClick={()=>setOpen(false)}>Kembali</button><div className="expanded-columns"><div className="expanded-art"><img src={`https://i.ytimg.com/vi/${track.id}/hqdefault.jpg`} alt={`Artwork ${track.title}`}/><h2>{track.title}</h2><p>{track.channel}</p></div><Lyrics/></div></section>
}
