import {useEffect,useRef,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {playTrack} from './playback';
type Track={id:string;title:string;channel?:string};
import {PlaylistPicker} from './PlaylistPicker';
export function Search(){
 const [query,setQuery]=useState(''),[tracks,setTracks]=useState<Track[]>([]),[error,setError]=useState(''),[busy,setBusy]=useState(false),[current,setCurrent]=useState('');
 const [adding,setAdding]=useState<Track|null>(null),[notice,setNotice]=useState('');
 const revision=useRef(0),playing=useRef(false);
 useEffect(()=>{
  const id=++revision.current;
  if(!query.trim()){setTracks([]);setBusy(false);setError('');return}
  setBusy(true);
  const timer=setTimeout(()=>{
   invoke<{entries?:Track[]}>('search',{query}).then(r=>{if(id===revision.current){setTracks(r.entries??[]);setError('')}}).catch(e=>{if(id===revision.current)setError(String(e))}).finally(()=>{if(id===revision.current)setBusy(false)})
  },450);
  return()=>{clearTimeout(timer);revision.current++}
 },[query]);
 async function play(track:Track){
  if(playing.current)return;
  playing.current=true;
  try{setCurrent(`Memuat ${track.title}`);await playTrack(track);setCurrent(track.title);setError('')}catch(e){setCurrent('');setError(String(e))}finally{playing.current=false}
 }
 return <section aria-label="Cari musik">
  <div className="discovery-heading"><span>Koleksi suara, tanpa batas suasana.</span><h2>Putar duniamu.</h2><p>Cari lagu favorit. Simpan penemuan baru. Nikmati waktumu.</p></div>
  <label htmlFor="search">Cari musik di YouTube</label>
  <input id="search" type="search" value={query} onChange={e=>setQuery(e.target.value)} placeholder="Judul lagu atau artis" style={{width:'100%'}}/>
  {busy&&<p role="status">Mencari…</p>}{error&&<p role="alert">{error}</p>}{notice&&<p role="status">{notice}</p>}
  <div className="track-grid">{tracks.map(t=><article key={t.id}>
   <img width="96" alt="" src={`https://i.ytimg.com/vi/${encodeURIComponent(t.id)}/mqdefault.jpg`}/><div style={{flex:1}}><strong>{t.title}</strong><p>{t.channel}</p></div>
   <button disabled={current.startsWith('Memuat ')} onClick={()=>void play(t)}>Putar</button><button onClick={()=>setAdding(t)}>Tambah ke playlist</button>
   <button onClick={async()=>{try{await invoke('enqueue',{track:{...t,channel:t.channel??null}});window.dispatchEvent(new Event('queue-changed'));setNotice(`${t.title} ditambahkan ke antrean`)}catch(e){setError(String(e))}}}>Tambah ke antrean</button>
  </article>)}</div>
  {adding&&<PlaylistPicker track={adding} onClose={()=>setAdding(null)} onAdded={()=>setNotice(`${adding.title} ditambahkan ke playlist`)}/>}
 </section>
}
