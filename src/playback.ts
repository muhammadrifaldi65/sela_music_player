import {invoke} from '@tauri-apps/api/core';
export type Track={id:string;title:string;channel?:string|null};
export type Entry={entry_id:number;track:Track};
export const playback={current:null as Entry|null,loading:false,session:'',recorded:false};
export const platform=invoke<string>('platform');
async function startEntry(entry:Entry){
 await invoke('play',{videoId:entry.track.id,entryId:entry.entry_id});
 await invoke('set_paused',{paused:false});
 await invoke('select_queue_entry',{id:entry.entry_id});
 playback.current=entry;
 playback.session=crypto.randomUUID();playback.recorded=false;
}
export async function playPlaylist(playlistId:number){
 if(playback.loading)return;
 const previous={current:playback.current,session:playback.session,recorded:playback.recorded};
 playback.loading=true;playback.current=null;playback.session='';playback.recorded=false;
 window.dispatchEvent(new Event('playback-changed'));
 let replaced=false;
 try{
  const entries=await invoke<Entry[]>('replace_queue_from_playlist',{playlistId});
  replaced=true;
  window.dispatchEvent(new Event('queue-changed'));
  await invoke('set_paused',{paused:true});
  await startEntry(entries[0]);
 }catch(error){
  if(!replaced)Object.assign(playback,previous);
  throw error;
 }finally{playback.loading=false;window.dispatchEvent(new Event('playback-changed'))}
}
export async function playTrack(track:Track,entryId?:number){
 if(playback.loading)return;
 playback.loading=true;window.dispatchEvent(new Event('playback-changed'));
 try{
  const id=entryId??await invoke<number>('enqueue',{track:{...track,channel:track.channel??null}});
  window.dispatchEvent(new Event('queue-changed'));
  await startEntry({entry_id:id,track});
 }finally{playback.loading=false;window.dispatchEvent(new Event('playback-changed'))}
}
