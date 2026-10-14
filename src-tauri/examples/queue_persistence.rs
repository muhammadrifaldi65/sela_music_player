#[path="../src/library.rs"] mod library;
#[path="../src/queue.rs"] mod queue;
fn main()->Result<(),Box<dyn std::error::Error>>{
 let path=std::env::temp_dir().join(format!("sela-queue-smoke-{}.sqlite",std::process::id()));
 let first;
 let second;
 {
 let mut db=rusqlite::Connection::open(&path)?;
 queue::initialize(&db)?;
 first=queue::append(&db,library::Track{id:"dQw4w9WgXcQ".into(),title:"First".into(),channel:None})?;
 second=queue::append(&db,library::Track{id:"dQw4w9WgXcQ".into(),title:"Second".into(),channel:None})?;
 queue::reorder(&mut db,vec![second,first])?;
 }
 {
 let db=rusqlite::Connection::open(&path)?;
 let entries=queue::entries(&db)?;
 assert_eq!(entries.iter().map(|e|e.entry_id).collect::<Vec<_>>(),vec![second,first]);
 assert_eq!(entries[0].track.title,"Second");
 }
 std::fs::remove_file(path)?;
 println!("Queue persisted across database close/reopen; duplicate identities and order preserved");
 Ok(())
}
