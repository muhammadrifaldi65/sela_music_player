#[path="../src/extractor.rs"]mod extractor;
fn main(){
 let runtime=tokio::runtime::Runtime::new().unwrap();runtime.block_on(async{
  let mut command=tokio::process::Command::new("cmd.exe");command.args(["/C","echo","hello"]);
  let bytes=extractor::run(&mut command,64,5).await.unwrap();assert_eq!(String::from_utf8(bytes).unwrap().trim(),"hello");
  let mut command=tokio::process::Command::new("cmd.exe");command.args(["/C","echo","too-long"]);
  assert_eq!(extractor::run(&mut command,3,5).await.unwrap_err(),"Respons media terlalu besar");
  println!("Extractor: valid output accepted; oversized output rejected");
 });
}
