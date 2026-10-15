import {createContext,useContext,useState,useEffect,type ReactNode} from 'react';
import {resolveLocale,translate,type Locale} from './translations';
let current:Locale=resolveLocale('system',navigator.language);
export const t=(key:string,values?:Record<string,string|number>)=>translate(current,key,values);
const Context=createContext({locale:current,t});
export function setLanguage(language:string){current=resolveLocale(language,navigator.language);document.documentElement.lang=current;window.dispatchEvent(new Event('language-changed'))}
export function I18nProvider({children}:{children:ReactNode}){const [locale,setLocale]=useState(current);useEffect(()=>{const changed=()=>setLocale(current);window.addEventListener('language-changed',changed);return()=>window.removeEventListener('language-changed',changed)},[]);return <Context.Provider value={{locale,t}}>{children}</Context.Provider>}
export const useTranslation=()=>useContext(Context);
export function errorText(error:unknown){const detail=error instanceof Error?error.message:String(error);const code=detail.match(/^(SETTINGS_[A-Z_]+|(?:lastfm|data)\.[a-z_.0-9]+)/)?.[0];if(code?.startsWith('lastfm.api.')&&t(code)===code)return t('Permintaan Last.fm ditolak (kode {code}).',{code:code.slice(11)});const player=detail.match(/^Pemutar menolak perintah \(([^)]+)\)$/);if(player)return t('Pemutar menolak perintah ({code})',{code:player[1]});if(detail.startsWith('Preferensi tidak dapat dibaca:'))return t('Preferensi tidak dapat dibaca')+detail.slice(26);if(code)return t(code)+(detail.length>code.length?' '+detail.slice(code.length):'');return `${t('Gagal menjalankan tindakan.')} ${t(detail)}`}
