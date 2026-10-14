use std::{ffi::{CString,c_char,c_void},path::Path};
use libloading::Library;
type Handle = *mut c_void;
pub struct Player { library: Library, handle: Handle }
// libmpv's client API is thread-safe; callers serialize access through a Mutex.
unsafe impl Send for Player {}
impl Player {
 pub fn new(path:&Path)->Result<Self,String>{unsafe{
  let library=Library::new(path).map_err(|e|e.to_string())?;
  let create=library.get::<unsafe extern "C" fn()->Handle>(b"mpv_create\0").map_err(|e|e.to_string())?;
  let handle=create(); if handle.is_null(){return Err("Gagal membuat pemutar".into())}
  let player=Self{library,handle};
  let set=player.library.get::<unsafe extern "C" fn(Handle,*const c_char,*const c_char)->i32>(b"mpv_set_option_string\0").map_err(|e|e.to_string())?;
  for (name,value) in [("vid","no"),("config","no"),("idle","yes"),("keep-open","yes"),("audio-display","no")]{let n=CString::new(name).unwrap();let v=CString::new(value).unwrap();if set(handle,n.as_ptr(),v.as_ptr())<0{return Err("Opsi pemutar ditolak".into())}}
  let init=player.library.get::<unsafe extern "C" fn(Handle)->i32>(b"mpv_initialize\0").map_err(|e|e.to_string())?;
  if init(handle)<0{return Err("Gagal menginisialisasi audio".into())} Ok(player)
 }}
 pub fn command(&self,args:&[&str])->Result<(),String>{
  let strings=args.iter().map(|a|CString::new(*a).map_err(|_|"Argumen tidak valid".to_string())).collect::<Result<Vec<_>,_>>()?;
  let mut pointers:Vec<_>=strings.iter().map(|s|s.as_ptr()).collect();pointers.push(std::ptr::null());
  unsafe{let call=self.library.get::<unsafe extern "C" fn(Handle,*const *const c_char)->i32>(b"mpv_command\0").map_err(|e|e.to_string())?;let result=call(self.handle,pointers.as_ptr());if result<0{Err(format!("Pemutar menolak perintah ({result})"))}else{Ok(())}}
 }
 pub fn number(&self, name: &str) -> Result<Option<f64>, String> {
  let name = CString::new(name).map_err(|_| "Properti tidak valid")?;
  let mut value = 0.0f64;
  unsafe {
   let get = self.library.get::<unsafe extern "C" fn(Handle,*const c_char,i32,*mut c_void)->i32>(b"mpv_get_property\0").map_err(|e|e.to_string())?;
   let result = get(self.handle,name.as_ptr(),5,(&mut value as *mut f64).cast());
   if result == -10 { return Ok(None); }
   if result < 0 { return Err(format!("Properti pemutar gagal ({result})")); }
  }
  Ok(value.is_finite().then_some(value))
 }
 pub fn flag(&self,name:&str)->Result<bool,String>{
  let name=CString::new(name).map_err(|_|"Properti tidak valid")?;
  let mut value=0i32;
  unsafe{let get=self.library.get::<unsafe extern "C" fn(Handle,*const c_char,i32,*mut c_void)->i32>(b"mpv_get_property\0").map_err(|e|e.to_string())?;
   let result=get(self.handle,name.as_ptr(),3,(&mut value as *mut i32).cast());
   if result == -10 {return Ok(false)}
   if result<0{return Err(format!("Properti pemutar gagal ({result})"))}
  }Ok(value!=0)
 }
}
impl Drop for Player{fn drop(&mut self){unsafe{if let Ok(destroy)=self.library.get::<unsafe extern "C" fn(Handle)>(b"mpv_terminate_destroy\0"){destroy(self.handle)}}}}
