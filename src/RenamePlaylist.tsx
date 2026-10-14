import {useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
export function RenamePlaylist({id,name,onSaved}:{id:number;name:string;onSaved:()=>Promise<void>}){
 const [editing,setEditing]=useState(false),[value,setValue]=useState(name),[error,setError]=useState('');
 return editing?<form onSubmit={async e=>{e.preventDefault();try{await invoke('rename_playlist',{id,name:value});await onSaved();window.dispatchEvent(new Event('library-changed'));setEditing(false);setError('')}catch(e){setError(String(e))}}}><label>Nama baru<input value={value} onChange={e=>setValue(e.target.value)} required maxLength={120}/></label><button>Simpan nama</button><button type="button" onClick={()=>setEditing(false)}>Batal</button>{error&&<p role="alert">{error}</p>}</form>:<button onClick={()=>{setValue(name);setEditing(true)}}>Ubah nama</button>
}
