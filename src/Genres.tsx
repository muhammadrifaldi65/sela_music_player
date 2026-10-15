import {useTranslation,t as tr,errorText} from './i18n';
import {createContext,useContext,useEffect,useRef,useState,type ReactNode} from 'react';
import {useSettings} from './settings';
import {invoke} from '@tauri-apps/api/core';
export type Genre={id:string;label:string;query:string};
type Preferences={genres:Genre[];selected:string[];edit:()=>void};
const PreferencesContext=createContext<Preferences|null>(null);
export function usePreferences(){const value=useContext(PreferencesContext);if(!value)throw new Error('Preferensi belum dimuat');return value}

function GenreForm({genres,initial,onSaved,onCancel}:{genres:Genre[];initial:string[];onSaved:(value:string[])=>void;onCancel?:()=>void}){
 const {t:tr,locale}=useTranslation();
 const [selected,setSelected]=useState(initial),[busy,setBusy]=useState(false),[error,setError]=useState('');
 const heading=useRef<HTMLHeadingElement>(null);
 useEffect(()=>{heading.current?.focus()},[]);
 async function save(event:React.FormEvent){
  event.preventDefault();if(busy||!selected.length)return;
  setBusy(true);setError('');
  try{onSaved(await invoke<string[]>('save_music_preferences',{genres:selected}))}
  catch(error){setError(tr('Selera musik belum tersimpan.')+' '+errorText(error));setBusy(false)}
 }
 return <form className="genre-form" onSubmit={event=>void save(event)} aria-busy={busy}>
  <h2 id="genre-heading" ref={heading} tabIndex={-1}>{onCancel?tr("Selera musik"):tr("Musik apa yang kamu suka?")}</h2>
  <p id="genre-help">{tr("Pilih satu atau lebih genre. Kamu bisa mengubahnya kapan saja lewat Selera musik.")}</p>
  <fieldset disabled={busy} aria-describedby="genre-help"><legend>{tr("Genre favorit")}</legend><div className="genre-options">{genres.map(genre=><label key={genre.id}><input type="checkbox" checked={selected.includes(genre.id)} onChange={event=>setSelected(current=>event.target.checked?[...current,genre.id]:current.filter(id=>id!==genre.id))}/><span>{genre.label}</span></label>)}</div></fieldset>
  <p className="genre-selection" role="status">{selected.length?tr('{count} genre dipilih',{count:selected.length}):tr("Pilih minimal satu genre untuk melanjutkan.")}</p>
  {error&&<p role="alert">{error}</p>}
  <div className="genre-actions">{onCancel&&<button type="button" disabled={busy} onClick={onCancel}>{tr("Batal")}</button>}<button type="submit" disabled={busy||!selected.length}>{busy?tr("Menyimpan…"):onCancel?tr("Simpan selera"):tr("Mulai mendengarkan")}</button></div>
 </form>
}
function GenreDialog({genres,selected,onSaved,onClose}:{genres:Genre[];selected:string[];onSaved:(value:string[])=>void;onClose:()=>void}){
 const {t:tr,locale}=useTranslation();
 const dialog=useRef<HTMLDialogElement>(null);
 useEffect(()=>{const previous=document.activeElement;dialog.current?.showModal();return()=>{dialog.current?.close();if(previous instanceof HTMLElement)previous.focus()}},[]);
 return <dialog ref={dialog} className="genre-dialog" aria-labelledby="genre-heading" onCancel={event=>{event.preventDefault();if(!dialog.current?.querySelector('form[aria-busy="true"]'))onClose()}}><GenreForm genres={genres} initial={selected} onSaved={onSaved} onCancel={onClose}/></dialog>;
}
export function PreferenceGate({children}:{children:ReactNode}){
 useSettings();
 const {t:tr,locale}=useTranslation();
 const [genres,setGenres]=useState<Genre[]>([]),[selected,setSelected]=useState<string[]|null|undefined>(undefined),[error,setError]=useState(''),[attempt,setAttempt]=useState(0),[editing,setEditing]=useState(false);
 useEffect(()=>{
  let active=true;setError('');
  void Promise.all([invoke<Genre[]>('music_genres'),invoke<string[]|null>('music_preferences')]).then(([genres,selected])=>{if(active){setGenres(genres);setSelected(selected)}}).catch(error=>{if(active)setError(tr('Selera musik tidak dapat dibaca.')+' '+errorText(error))});
  return()=>{active=false};
 },[attempt]);
 if(selected===undefined)return <main className="genre-start"><section>{error?<><h2>{tr("Preferensi belum tersedia")}</h2><p role="alert">{error}</p><button onClick={()=>setAttempt(value=>value+1)}>{tr("Coba lagi")}</button></>:<p role="status">{tr("Memuat selera musik…")}</p>}</section></main>;
 if(selected===null)return <main className="genre-start"><section><p className="genre-brand">Sela</p><GenreForm genres={genres} initial={[]} onSaved={setSelected}/></section></main>;
 return <PreferencesContext.Provider value={{genres,selected,edit:()=>setEditing(true)}}>{children}{editing&&<GenreDialog genres={genres} selected={selected} onSaved={value=>{setSelected(value);setEditing(false)}} onClose={()=>setEditing(false)}/>}</PreferencesContext.Provider>;
}
