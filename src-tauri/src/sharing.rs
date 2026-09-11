//! Explicit, read-only classroom file sharing. No filesystem path is accepted over HTTP.
use std::{collections::BTreeMap, io::{Read,Write}, net::{Ipv4Addr,TcpListener,TcpStream,UdpSocket}, path::PathBuf, sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}}, time::Duration};
use tauri::{Manager,WebviewUrl,WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;
type Result<T> = std::result::Result<T,String>;
#[derive(Clone,serde::Serialize)]
pub struct Item { id:String, name:String, size:u64, #[serde(skip)] path:PathBuf }
struct Server { stop:Arc<AtomicBool>, thread:Option<std::thread::JoinHandle<()>>, url:String }
impl Drop for Server { fn drop(&mut self){self.stop.store(true,Ordering::SeqCst);if let Some(t)=self.thread.take(){let _=t.join();}} }
#[derive(Default)]
pub struct ShareState { server:Mutex<Option<Server>>, files:Arc<Mutex<BTreeMap<String,Item>>>, live:Arc<Live> }
#[derive(Default)]
struct Live { enabled:AtomicBool, generation:std::sync::atomic::AtomicU64, frame:Mutex<Vec<u8>>, error:Mutex<String> }
impl Live { fn stop(&self){self.enabled.store(false,Ordering::SeqCst);self.generation.fetch_add(1,Ordering::SeqCst);self.frame.lock().unwrap().clear();} }
#[derive(serde::Serialize)]
pub struct Status { running:bool, url:String, address:String, qr:String, files:Vec<Item>, live:bool, live_error:String }
fn default_ip()->String { (||{let s=UdpSocket::bind("0.0.0.0:0").ok()?;s.connect("192.0.2.1:80").ok()?;Some(s.local_addr().ok()?.ip().to_string())})().unwrap_or_default() }
fn allowed(ip:Ipv4Addr)->bool {ip.is_private()||ip.is_link_local()||ip.is_loopback()}
fn escape(s:&str)->String {s.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;").replace('"',"&quot;").replace('\'',"&#39;")}
fn page(files:&BTreeMap<String,Item>)->String {
 let rows=files.values().map(|f|format!("<li><a href=\"file/{}\">{}</a><small>{:.1} MB</small></li>",f.id,escape(&f.name),f.size as f64/1_048_576.)).collect::<String>();
 format!("<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Pointory · Class materials</title><style>body{{font:17px system-ui;background:#edf3fa;color:#152840;max-width:720px;margin:8vh auto;padding:24px}}h1{{color:#2368cc}}li{{background:white;padding:20px;margin:12px 0;border-radius:14px;overflow-wrap:anywhere}}ul{{padding:0;list-style:none}}a{{color:#1759b5}}small{{display:block;margin-top:8px;color:#526278}}</style><h1>Pointory</h1><h2>Class materials · 수업 자료</h2><p>Choose a file to download. 파일을 선택해 내려받으세요.</p><ul>{rows}</ul><p>{}</p><p><a href=\"./\">Refresh · 새로고침</a></p><footer>Shared by this PC while Pointory sharing is on.<br>강사 PC에서 공유를 켠 동안 이용할 수 있습니다.</footer></html>",if files.is_empty(){"No materials yet. 아직 공유된 자료가 없습니다."}else{""})
}
struct Connection(TcpStream);
impl Read for Connection { fn read(&mut self,buf:&mut [u8])->std::io::Result<usize>{let start=std::time::Instant::now();loop{match self.0.read(buf){Err(e) if e.kind()==std::io::ErrorKind::WouldBlock&&start.elapsed()<Duration::from_secs(2)=>std::thread::sleep(Duration::from_millis(5)),result=>return result}}} }
impl Write for Connection { fn write(&mut self,buf:&[u8])->std::io::Result<usize>{let start=std::time::Instant::now();loop{match self.0.write(buf){Err(e) if e.kind()==std::io::ErrorKind::WouldBlock&&start.elapsed()<Duration::from_secs(2)=>std::thread::sleep(Duration::from_millis(5)),result=>return result}}} fn flush(&mut self)->std::io::Result<()>{Ok(())} }
fn header(s:&mut Connection,status:&str,kind:&str,len:u64,extra:&str)->std::io::Result<()> {
 write!(s,"HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {len}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; img-src 'self' blob:; script-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'\r\n{extra}\r\n")
}
fn reply(s:&mut Connection,status:&str,text:&str){let _=header(s,status,"text/html; charset=utf-8",text.len() as u64,"");let _=s.write_all(text.as_bytes());}
fn handle(stream:TcpStream,token:&str,host:&str,files:&Mutex<BTreeMap<String,Item>>,stop:&AtomicBool,live:&Live){
 let _=stream.set_nonblocking(true);let mut stream=Connection(stream);
 let mut bytes=Vec::new();let mut buf=[0u8;1024];let started=std::time::Instant::now();
 loop {if stop.load(Ordering::SeqCst)||started.elapsed()>Duration::from_secs(3){return;}match stream.read(&mut buf){Ok(0)|Err(_)=>return,Ok(n)=>bytes.extend_from_slice(&buf[..n])}if bytes.len()>8192{return;}if bytes.windows(4).any(|w|w==b"\r\n\r\n"){break;}}
 let Ok(req)=std::str::from_utf8(&bytes) else{return;};let mut lines=req.split("\r\n");let parts=lines.next().unwrap_or("").split_whitespace().collect::<Vec<_>>();
 if parts.len()!=3||parts[0]!="GET" {reply(&mut stream,"405 Method Not Allowed","Read-only sharing");return;}
 let hosts=lines.filter_map(|l|l.split_once(':')).filter(|(k,_)|k.eq_ignore_ascii_case("host")).map(|(_,v)|v.trim()).collect::<Vec<_>>();
 if hosts!=[host]{reply(&mut stream,"403 Forbidden","Invalid host");return;}
 let prefix=format!("/{token}/");let Some(route)=parts[1].strip_prefix(&prefix) else{reply(&mut stream,"404 Not Found","Not found");return;};
 if stop.load(Ordering::SeqCst){return;}
 if route=="viewer.js"{let js=include_str!("../resources/share-viewer.js");let _=header(&mut stream,"200 OK","text/javascript; charset=utf-8",js.len() as u64,"");let _=stream.write_all(js.as_bytes());return;}
 if route=="live.jpg"{let frame=live.frame.lock().unwrap();if !live.enabled.load(Ordering::SeqCst)||frame.is_empty(){reply(&mut stream,"404 Not Found","Live view is off or preparing");return;}let _=header(&mut stream,"200 OK","image/jpeg",frame.len() as u64,"");let _=stream.write_all(&frame);return;}
 if route.is_empty(){let html=page(&files.lock().unwrap()).replace("<ul>","<section><h2>Live view · 실시간 화면</h2><p id=\"liveStatus\">Waiting · 대기</p><img id=\"live\" alt=\"Instructor screen · 강사 화면\" style=\"width:100%;display:none\"><script src=\"viewer.js\" defer></script></section><ul>");reply(&mut stream,"200 OK",&html);return;}
 let item=route.strip_prefix("file/").and_then(|id|files.lock().ok()?.get(id).cloned());
 let Some(item)=item else{reply(&mut stream,"404 Not Found","Not found");return;};
 let Ok(mut file)=std::fs::File::open(&item.path) else{reply(&mut stream,"404 Not Found","File unavailable");return;};
 let Ok(meta)=file.metadata() else{return;};if !meta.is_file(){reply(&mut stream,"404 Not Found","File unavailable");return;}
 let encoded=item.name.as_bytes().iter().map(|b|format!("%{b:02X}")).collect::<String>();
 if header(&mut stream,"200 OK","application/octet-stream",meta.len(),&format!("Content-Disposition: attachment; filename=\"download\"; filename*=UTF-8''{encoded}\r\n")).is_err(){return;}
 let mut remaining=meta.len();let mut data=[0u8;65536];
 while remaining>0&&!stop.load(Ordering::SeqCst){if !files.lock().unwrap().contains_key(&item.id){break;}let size=remaining.min(data.len() as u64) as usize;match file.read(&mut data[..size]){Ok(0)|Err(_)=>break,Ok(n)=>{if stream.write_all(&data[..n]).is_err(){break;}remaining-=n as u64;}}}
}
fn start(ip:Ipv4Addr,port:u16,files:Arc<Mutex<BTreeMap<String,Item>>>,live:Arc<Live>)->Result<Server>{
 if !allowed(ip){return Err("Choose a private LAN IPv4 address. 사설 네트워크 IPv4 주소를 선택하세요.".into());}
 let listener=TcpListener::bind((ip,port)).map_err(|e|e.to_string())?;listener.set_nonblocking(true).map_err(|e|e.to_string())?;
 let host=listener.local_addr().map_err(|e|e.to_string())?.to_string();let token=uuid::Uuid::new_v4().simple().to_string();let url=format!("http://{host}/{token}/");let stop=Arc::new(AtomicBool::new(false));let flag=stop.clone();
 let thread=std::thread::spawn(move||{
  let mut workers:Vec<std::thread::JoinHandle<()>>=vec![];
  while !flag.load(Ordering::SeqCst){
   let mut i=0;while i<workers.len(){if workers[i].is_finished(){let _=workers.swap_remove(i).join();}else{i+=1;}}
   match listener.accept(){
    Ok((stream,peer)) if workers.len()<8&&matches!(peer.ip(),std::net::IpAddr::V4(a) if allowed(a))=>{
     let files=files.clone();let flag=flag.clone();let token=token.clone();let host=host.clone();let live=live.clone();
     workers.push(std::thread::spawn(move||handle(stream,&token,&host,&files,&flag,&live)));
    },
    Ok(_)=>{},
    Err(e) if e.kind()==std::io::ErrorKind::WouldBlock=>std::thread::sleep(Duration::from_millis(40)),
    Err(_)=>break,
   }
  }
  for worker in workers{let _=worker.join();}
 });
 Ok(Server{stop,thread:Some(thread),url})
}
#[tauri::command]
pub fn share_status(state:tauri::State<ShareState>)->Result<Status>{
 let server=state.server.lock().map_err(|e|e.to_string())?;let url=server.as_ref().map(|s|s.url.clone()).unwrap_or_default();
 let qr=if url.is_empty(){String::new()}else{qrcode::QrCode::new(url.as_bytes()).map_err(|e|e.to_string())?.render::<qrcode::render::svg::Color>().min_dimensions(220,220).build()};
 Ok(Status{running:server.is_some(),url,address:default_ip(),qr,files:state.files.lock().map_err(|e|e.to_string())?.values().cloned().collect(),live:state.live.enabled.load(Ordering::SeqCst),live_error:state.live.error.lock().unwrap().clone()})
}
#[tauri::command]
pub fn share_start(address:String,state:tauri::State<ShareState>)->Result<()> {let ip=address.parse::<Ipv4Addr>().map_err(|e|e.to_string())?;let mut s=state.server.lock().map_err(|e|e.to_string())?;if s.is_none(){*s=Some(start(ip,0,state.files.clone(),state.live.clone())?);}Ok(())}
#[tauri::command]
pub async fn share_stop(app:tauri::AppHandle)->Result<()> {tauri::async_runtime::spawn_blocking(move||{let state=app.state::<ShareState>();state.live.stop();let server=state.server.lock().map_err(|e|e.to_string())?.take();drop(server);Ok(())}).await.map_err(|e|e.to_string())?}
#[tauri::command]
pub async fn share_add(app:tauri::AppHandle)->Result<()> {
 let Some(paths)=app.dialog().file().set_title("Share materials · 자료 선택").blocking_pick_files() else{return Ok(());};
 let state=app.state::<ShareState>();let mut files=state.files.lock().map_err(|e|e.to_string())?;
 for path in paths{let path=path.into_path().map_err(|e|e.to_string())?.canonicalize().map_err(|e|e.to_string())?;let meta=path.metadata().map_err(|e|e.to_string())?;if !meta.is_file()||files.values().any(|f|f.path==path){continue;}if files.len()>=100{return Err("Maximum 100 files · 최대 100개".into());}let id=uuid::Uuid::new_v4().simple().to_string();let name=path.file_name().unwrap_or_default().to_string_lossy().into_owned();files.insert(id.clone(),Item{id,name,size:meta.len(),path});}Ok(())
}
#[tauri::command]
pub fn share_remove(id:String,state:tauri::State<ShareState>)->Result<()> {state.files.lock().map_err(|e|e.to_string())?.remove(&id);Ok(())}
#[tauri::command]
pub async fn share_open(app:tauri::AppHandle)->Result<()> {if let Some(w)=app.get_webview_window("sharing"){super::macos::set_level(&w)?;w.show().map_err(|e|e.to_string())?;w.set_focus().map_err(|e|e.to_string())?;}else{let w=WebviewWindowBuilder::new(&app,"sharing",WebviewUrl::App("sharing.html".into())).title("Pointory · 자료 공유 / Share materials").inner_size(540.,740.).build().map_err(|e|e.to_string())?;super::macos::set_level(&w)?;}Ok(())}

#[tauri::command]
pub async fn share_live(app:tauri::AppHandle,enabled:bool)->Result<()> {
 let share=app.state::<ShareState>();share.live.stop();share.live.error.lock().unwrap().clear();
 if !enabled{return Ok(());}
 if share.server.lock().unwrap().is_none(){return Err("Start sharing first · 먼저 자료 공유를 시작하세요.".into());}
 let display=app.state::<super::Shared>().lock().map_err(|e|e.to_string())?.selected().ok_or("Select a monitor · 모니터를 선택하세요.")?.clone();
 let live=share.live.clone();let generation=live.generation.load(Ordering::SeqCst);live.enabled.store(true,Ordering::SeqCst);
 std::thread::spawn(move||{
  let result=(||->Result<()>{
   super::capture::ensure_capture_permission()?;
   let monitor=super::capture::monitor_for_display(&display)?;
   while live.enabled.load(Ordering::SeqCst)&&live.generation.load(Ordering::SeqCst)==generation {
    let selected=app.state::<super::Shared>().lock().map_err(|e|e.to_string())?.selected().map(|d|d.id.clone());
    if selected.as_deref()!=Some(&display.id){return Err("Monitor changed. Restart live view · 모니터 변경: 화면 공유를 다시 켜세요.".into());}
    let image=monitor.capture_image().map_err(|e|format!("Screen capture failed: {e}"))?;
    let mut image=xcap::image::DynamicImage::ImageRgba8(image).thumbnail(1280,720).to_rgb8();super::spotlight::composite(&app,&display,&mut image);let mut bytes=Vec::new();
    xcap::image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes,70).encode_image(&image).map_err(|e|e.to_string())?;
    let mut frame=live.frame.lock().unwrap();if live.enabled.load(Ordering::SeqCst)&&live.generation.load(Ordering::SeqCst)==generation{*frame=bytes;}drop(frame);
    std::thread::sleep(Duration::from_millis(500));
   }Ok(())
  })();
  if live.generation.load(Ordering::SeqCst)==generation{if let Err(e)=result{*live.error.lock().unwrap()=e;live.stop();}}
 });Ok(())
}

#[cfg(test)] mod tests {
 use super::*;
 fn get(url:&str,path:Option<&str>,host_override:Option<&str>)->String{let target=url.strip_prefix("http://").unwrap();let (host,route)=target.split_once('/').unwrap();let mut s=TcpStream::connect(host).unwrap();s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();write!(s,"GET {} HTTP/1.1\r\nHost: {}\r\n\r\n",path.map(str::to_string).unwrap_or_else(||format!("/{route}")),host_override.unwrap_or(host)).unwrap();let mut out=String::new();s.read_to_string(&mut out).unwrap();out}
 #[test] fn shares_only_selected_files_and_revokes_access(){let dir=std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());std::fs::create_dir(&dir).unwrap();let path=dir.join("lesson.txt");std::fs::write(&path,"lesson contents").unwrap();let files=Arc::new(Mutex::new(BTreeMap::new()));files.lock().unwrap().insert("abc".into(),Item{id:"abc".into(),name:"<lesson>.txt".into(),size:15,path});let server=start(Ipv4Addr::LOCALHOST,0,files.clone(),Arc::new(Live::default())).unwrap();let url=server.url.clone();assert!(get(&url,None,None).contains("&lt;lesson&gt;.txt"));assert!(get(&url,Some("/"),None).starts_with("HTTP/1.1 404"));assert!(get(&url,None,Some("evil.example")).starts_with("HTTP/1.1 403"));assert!(get(&(url.clone()+"file/abc"),None,None).contains("lesson contents"));assert!(get(&(url.clone()+"file/../lesson.txt"),None,None).starts_with("HTTP/1.1 404"));files.lock().unwrap().clear();assert!(get(&(url.clone()+"file/abc"),None,None).starts_with("HTTP/1.1 404"));let addr=url.strip_prefix("http://").unwrap().split('/').next().unwrap().to_string();drop(server);assert!(TcpStream::connect(addr).is_err());std::fs::remove_dir_all(dir).unwrap();}
 #[test] fn rejects_public_bind(){assert!(!allowed("8.8.8.8".parse().unwrap()));assert!(allowed("192.168.1.2".parse().unwrap()));}
 #[test] fn live_frame_is_revoked_when_stopped(){let live=Arc::new(Live::default());live.enabled.store(true,Ordering::SeqCst);*live.frame.lock().unwrap()=b"test-frame".to_vec();let server=start(Ipv4Addr::LOCALHOST,0,Arc::new(Mutex::new(BTreeMap::new())),live.clone()).unwrap();let url=server.url.clone()+"live.jpg";assert!(get(&url,None,None).contains("test-frame"));live.stop();assert!(get(&url,None,None).starts_with("HTTP/1.1 404"));}
 fn synthetic_fixture_frame(index:u32)->Vec<u8>{
  // Entirely authored RGB pixels: a moving stripe and binary frame counter.
  // No monitor, window, screenshot, or external image is read by this path.
  let stripe=(index%640)*37%640;
  let image=xcap::image::RgbImage::from_fn(640,360,|x,y|{
   let pixel=if y<40 {
    if (index>>(x/40))&1==1 {[240,192,64]}else{[32,48,80]}
   }else if (x+640-stripe)%640<48 {[72,216,168]}
   else if (x/40+y/40)%2==0 {[40,72,120]}else{[28,48,88]};
   xcap::image::Rgb(pixel)
  });
  let mut bytes=vec![];xcap::image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes,70).encode_image(&image).unwrap();bytes
 }
 #[test] fn synthetic_fixture_frames_decode_and_change(){
  let first=synthetic_fixture_frame(0);let next=synthetic_fixture_frame(1);
  let first_image=xcap::image::load_from_memory(&first).unwrap().to_rgb8();
  let next_image=xcap::image::load_from_memory(&next).unwrap().to_rgb8();
  assert_eq!(first_image.dimensions(),(640,360));assert_eq!(next_image.dimensions(),(640,360));
  assert_ne!(first_image.as_raw(),next_image.as_raw());
  assert_eq!(first,synthetic_fixture_frame(0));
 }
 #[test] #[ignore = "Manual browser fixture; no desktop capture"]
 fn browser_fixture(){
  let dynamic=std::env::var("POINTORY_FIXTURE_DYNAMIC").as_deref()==Ok("1");
  let live=Arc::new(Live::default());
  let bytes=if dynamic {synthetic_fixture_frame(0)}else{
   let image=xcap::image::open(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../Wiki/assets/pointory-overview.jpg")).unwrap().to_rgb8();
   let mut bytes=vec![];xcap::image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes,70).encode_image(&image).unwrap();bytes
  };
  *live.frame.lock().unwrap()=bytes;live.enabled.store(true,Ordering::SeqCst);
  let files=Arc::new(Mutex::new(BTreeMap::new()));let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../README.md");files.lock().unwrap().insert("sample".into(),Item{id:"sample".into(),name:"Pointory-example-readme.md".into(),size:path.metadata().unwrap().len(),path});
  // Explicitly opt into a private LAN bind for a cross-device fixture. The
  // production server's address validation still rejects public interfaces.
  let bind=std::env::var("POINTORY_FIXTURE_BIND").map(|value|value.parse::<Ipv4Addr>().expect("Invalid fixture IPv4 address")).unwrap_or(Ipv4Addr::LOCALHOST);
  let port=std::env::var("POINTORY_FIXTURE_PORT").map(|value|value.parse::<u16>().expect("Invalid fixture port")).unwrap_or(0);
  let seconds=std::env::var("POINTORY_FIXTURE_SECONDS").ok().and_then(|value|value.parse::<u64>().ok()).unwrap_or(300).clamp(5,300);
  let server=start(bind,port,files,live.clone()).unwrap();println!("BROWSER_FIXTURE_URL={}",server.url);
  let started=std::time::Instant::now();
  if dynamic {
   for frame in 1..seconds*2 {
    std::thread::sleep((started+Duration::from_millis(frame*500)).saturating_duration_since(std::time::Instant::now()));
    *live.frame.lock().unwrap()=synthetic_fixture_frame(frame as u32);
   }
  }
  std::thread::sleep((started+Duration::from_secs(seconds)).saturating_duration_since(std::time::Instant::now()));
 }
}
