import {t,errorText} from './i18n';
import {useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {usePreferences} from './Genres';
import {balanceTracks} from './genreTracks';
import type {Track} from './playback';

export function useGenreDiscovery(){
 const {genres,selected,edit}=usePreferences();
 const [items,setItems]=useState<Track[]>([]),[errors,setErrors]=useState<string[]>([]),[loading,setLoading]=useState(true),[attempt,setAttempt]=useState(0);
 useEffect(()=>{
  let active=true;
  setItems([]);setErrors([]);setLoading(true);
  const chosen=selected.map(id=>genres.find(genre=>genre.id===id)!);
  // Two workers keep extractor processes bounded even with all twelve genres selected.
  const groups:Track[][]=chosen.map(()=>[]),failures:string[]=chosen.map(()=> '');let next=0;
  async function worker(){
   while(active&&next<chosen.length){
    const index=next++,genre=chosen[index];
    try{
     const result=await invoke<{entries?:unknown[]}>('search',{query:genre.query});
     if(!Array.isArray(result.entries))throw new Error(t('Respons pencarian tidak valid.'));
     groups[index]=result.entries.flatMap(entry=>{
      if(!entry||typeof entry!=='object')return [];
      const row=entry as Record<string,unknown>;
      if(typeof row.id!=='string'||!/^[\w-]{11}$/.test(row.id)||typeof row.title!=='string'||!row.title.trim())return [];
      return [{id:row.id,title:row.title,channel:typeof row.channel==='string'?row.channel:typeof row.uploader==='string'?row.uploader:null}];
     });
     if(!groups[index].length)failures[index]=`${genre.label}: ${t('Tidak ada hasil pencarian yang dapat diputar.')}`;
    }catch(error){failures[index]=`${genre.label}: ${errorText(error)}`}
    if(active){setItems(balanceTracks(groups));setErrors(failures.filter(Boolean))}
   }
  }
  void Promise.all([worker(),worker()]).finally(()=>{if(active)setLoading(false)});
  return()=>{active=false};
 },[genres,selected,attempt]);
 return {items,errors,loading,retry:()=>setAttempt(value=>value+1),edit,labels:genres.filter(genre=>selected.includes(genre.id)).map(genre=>genre.label).join(', ')};
}
