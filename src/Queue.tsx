import {useTranslation,t as tr,errorText} from './i18n';
import {useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {playTrack,type Entry} from './playback';
export function Queue(){
 const {t:tr,locale}=useTranslation();
 const [entries,setEntries]=useState<Entry[]>([]),[error,setError]=useState('');
 async function refresh(){try{setEntries(await invoke('queue_entries'))}catch(e){setError(errorText(e))}}
 useEffect(()=>{void refresh();const changed=()=>void refresh();window.addEventListener('queue-changed',changed);return()=>window.removeEventListener('queue-changed',changed)},[]);
 async function move(from:number,to:number){if(from===to)return;const next=[...entries];const [entry]=next.splice(from,1);next.splice(to,0,entry);try{await invoke('reorder_queue',{ids:next.map(e=>e.entry_id)});window.dispatchEvent(new Event('queue-changed'))}catch(e){setError(errorText(e))}}
 return <section className="queue-panel" aria-label={tr("Antrean")}><h2>{tr("Antrean")}</h2>{entries.length===0&&<p>{tr("Tambahkan lagu dari pencarian.")}</p>}<ol>{entries.map((e,index)=><li key={e.entry_id} draggable onDragStart={event=>event.dataTransfer.setData('text/plain',String(e.entry_id))} onDragOver={event=>event.preventDefault()} onDrop={event=>{event.preventDefault();const from=entries.findIndex(e=>e.entry_id===Number(event.dataTransfer.getData('text/plain')));if(from>=0)void move(from,index)}}>{e.track.title} <button onClick={()=>void playTrack(e.track,e.entry_id).catch(e=>setError(errorText(e)))}>{tr("Putar dari antrean")}</button><button disabled={index===0} onClick={()=>void move(index,index-1)}>{tr("Naik")}</button><button disabled={index===entries.length-1} onClick={()=>void move(index,index+1)}>{tr("Turun")}</button><button onClick={async()=>{try{await invoke('remove_queue_entry',{id:e.entry_id});window.dispatchEvent(new Event('queue-changed'))}catch(e){setError(errorText(e))}}}>{tr('Hapus dari antrean')}</button></li>)}</ol>{error&&<p role="alert">{error}</p>}</section>
}
