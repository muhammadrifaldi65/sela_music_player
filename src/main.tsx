import React,{useEffect,useState} from 'react';
import {createRoot} from 'react-dom/client';
import {invoke} from '@tauri-apps/api/core';
import './styles.css';
import {Search} from './Search';
import {Transport} from './Transport';
import {PlaylistTracks} from './PlaylistTracks';
import {RenamePlaylist} from './RenamePlaylist';
import {Queue} from './Queue';
import {Home,History} from './Discovery';
import {NowPlaying} from './NowPlaying';
import {Discord} from './Discord';
import {PreferenceGate} from './Genres';
type Playlist={id:number;name:string};
function App(){
 const [compact,setCompact]=useState(()=>localStorage.getItem('sidebar-compact')==='true');
 const [items,setItems]=useState<Playlist[]>([]),[name,setName]=useState(''),[error,setError]=useState(''),[page,setPage]=useState('home'),[queueOpen,setQueueOpen]=useState(false),[selected,setSelected]=useState<number|null>(null);
 async function refresh(){const lists=await invoke<Playlist[]>('playlists');setItems(lists);setSelected(id=>lists.some(p=>p.id===id)?id:null)}
 useEffect(()=>{const changed=()=>void refresh().catch(e=>setError(String(e)));changed();window.addEventListener('library-changed',changed);return()=>window.removeEventListener('library-changed',changed)},[]);
 useEffect(()=>{const toggle=()=>setQueueOpen(v=>!v);const key=(event:KeyboardEvent)=>{if(event.key==='Escape')setQueueOpen(false);if(event.ctrlKey&&event.key.toLowerCase()==='k'){event.preventDefault();setPage('search');requestAnimationFrame(()=>document.getElementById('search')?.focus())}};window.addEventListener('keydown',key);window.addEventListener('toggle-queue',toggle);return()=>{window.removeEventListener('keydown',key);window.removeEventListener('toggle-queue',toggle)}},[]);
 useEffect(()=>{const open=(event:Event)=>{setSelected((event as CustomEvent<number>).detail);setPage('library')};window.addEventListener('open-playlist',open);return()=>window.removeEventListener('open-playlist',open)},[]);
 function openPlaylist(id:number){setSelected(id);setPage('library')}
 async function create(e:React.FormEvent){e.preventDefault();try{const id=await invoke<number>('create_playlist',{name});setName('');setError('');await refresh();openPlaylist(id);window.dispatchEvent(new Event('library-changed'))}catch(e){setError(String(e))}}
 async function remove(playlist:Playlist){if(!confirm(`Hapus playlist “${playlist.name}”?`))return;try{await invoke('delete_playlist',{id:playlist.id});await refresh();window.dispatchEvent(new Event('library-changed'))}catch(e){setError(String(e))}}
 const list=items.find(p=>p.id===selected);
 return <><div className={compact?'shell sidebar-compact':'shell'}><aside><div className="brand-row"><button className="nav-toggle" aria-label="Ringkas navigasi" aria-expanded={!compact} onClick={()=>{setCompact(!compact);localStorage.setItem('sidebar-compact',String(!compact))}}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 6h16M4 12h16M4 18h16"/></svg></button><h1>Sela<span>By PGM Community</span></h1></div><nav aria-label="Navigasi">{Object.entries({home:'Home',search:'Cari musik',history:'Riwayat',library:'Playlist saya'}).map(([id,label])=><button key={id} title={label} aria-label={label} onClick={()=>{setPage(id);if(id==='library')setSelected(null)}} aria-current={page===id&&(id!=='library'||!list)?'page':undefined}><svg viewBox="0 0 24 24" aria-hidden="true"><path d={({home:'M3 10 12 3l9 7v11h-6v-7H9v7H3Z',search:'M21 21l-6-6M17 10a7 7 0 1 1-14 0 7 7 0 0 1 14 0',history:'M3 4v6h6M3 10a9 9 0 1 1 1 8M12 7v5l3 2',library:'M4 4v16M9 4v16M14 5l4-1 4 15-4 1Z'} as Record<string,string>)[id]}/></svg><span className="nav-label">{label}</span></button>)}</nav><nav className="sidebar-playlists" aria-label="Playlist saya">{items.map(p=><button key={p.id} title={p.name} onClick={()=>openPlaylist(p.id)} aria-current={page==='library'&&selected===p.id?'page':undefined}>{p.name}</button>)}</nav><p className="privacy">Tersimpan di perangkat ini.<br/>Tanpa akun. Tanpa server.</p></aside><main><header className="app-header"><span>Musik, di ruangmu.</span><Discord/></header><div hidden={page!=='home'}><Home/></div><div hidden={page!=='search'}><Search/></div><div hidden={page!=='history'}><History/></div><div hidden={page!=='library'}>
 {error&&<p role="alert">{error}</p>}
 {list?<><button onClick={()=>setSelected(null)}>Kembali ke playlist</button><PlaylistTracks key={list.id} playlist={list}/><div className="playlist-management"><RenamePlaylist key={`${list.id}:${list.name}`} id={list.id} name={list.name} onSaved={refresh}/><button onClick={()=>void remove(list)}>Hapus playlist</button></div></>:<><h2>Playlist saya</h2><form onSubmit={create}><label htmlFor="name">Nama playlist</label><div className="form-row"><input id="name" value={name} onChange={e=>setName(e.target.value)} required maxLength={120} placeholder="Perjalanan pulang"/><button type="submit">Buat playlist</button></div></form>{!items.length&&<p>Belum ada playlist. Buat koleksi pertama untuk menyimpan lagu favorit.</p>}<section aria-label="Playlist" className="playlists">{items.map(p=><article key={p.id}><button className="playlist-card-open" onClick={()=>openPlaylist(p.id)} aria-label={`Buka playlist ${p.name}`}><span className="cover" aria-hidden="true">{p.name.slice(0,1)}</span><h3>{p.name}</h3><span>Buka playlist</span></button><RenamePlaylist key={`${p.id}:${p.name}`} id={p.id} name={p.name} onSaved={refresh}/><button onClick={()=>void remove(p)}>Hapus playlist</button></article>)}</section></>}
 </div></main></div><div className={queueOpen?'queue-container open':'queue-container'}><button className="queue-close" onClick={()=>setQueueOpen(false)}>Tutup antrean</button><Queue/></div><Transport/></>
}
createRoot(document.getElementById('root')!).render(<PreferenceGate><App/><NowPlaying/></PreferenceGate>);
