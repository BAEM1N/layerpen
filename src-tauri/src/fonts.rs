use std::{collections::BTreeSet,path::PathBuf,sync::OnceLock,io::Read};
use tauri::Emitter;
use tauri_plugin_dialog::DialogExt;
use base64::Engine;
use sha2::{Digest,Sha256};
const LIMIT:u64=32*1024*1024;
#[derive(serde::Serialize)]
pub struct Font {id:String,family:String,name:String}
fn directory(app:&tauri::AppHandle)->Result<PathBuf,String>{Ok(super::settings_path(app)?.parent().ok_or("Settings directory unavailable")?.join("fonts"))}
fn names(data:Vec<u8>)->Vec<String>{let mut db=fontdb::Database::new();db.load_font_data(data);db.faces().flat_map(|f|f.families.iter().map(|(name,_)|name.clone())).filter(|n|super::valid_font(n)).collect::<BTreeSet<_>>().into_iter().collect()}
fn read_font(path:&std::path::Path)->Result<Vec<u8>,String>{
 let mut bytes=Vec::new();std::fs::File::open(path).map_err(|e|e.to_string())?.take(LIMIT+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
 if bytes.len() as u64>LIMIT{return Err("Font file must be smaller than 32 MB".into());}Ok(bytes)
}
fn valid_id(id:&str)->bool{id.len()==64&&id.bytes().all(|b|b.is_ascii_hexdigit())}
fn asset(id:String,bytes:Vec<u8>)->Result<Font,String>{let name=names(bytes).into_iter().next().ok_or("Invalid or unsupported font file")?;Ok(Font{family:format!("OnPenFont-{id}"),id,name})}
#[tauri::command]
pub async fn font_assets(app:tauri::AppHandle)->Result<Vec<Font>,String>{
 let dir=directory(&app)?;tauri::async_runtime::spawn_blocking(move||{
 let mut result=vec![];if !dir.exists(){return Ok(result);}
 for entry in std::fs::read_dir(dir).map_err(|e|e.to_string())?.flatten(){let path=entry.path();let id=path.file_stem().and_then(|s|s.to_str()).unwrap_or("");if path.extension().is_some_and(|s|s=="ttf")&&valid_id(id){if let Ok(font)=read_font(&path).and_then(|data|asset(id.into(),data)){result.push(font);}}}result.sort_by(|a,b|a.name.cmp(&b.name));Ok(result)
 }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn system_fonts()->Result<Vec<String>,String>{
 tauri::async_runtime::spawn_blocking(||{static FONTS:OnceLock<Vec<String>>=OnceLock::new();FONTS.get_or_init(||{let mut db=fontdb::Database::new();db.load_system_fonts();db.faces().flat_map(|f|f.families.iter().map(|(name,_)|name.clone())).filter(|n|super::valid_font(n)).collect::<BTreeSet<_>>().into_iter().collect()}).clone()}).await.map_err(|e|e.to_string())
}
#[tauri::command]
pub async fn font_data(app:tauri::AppHandle,id:String)->Result<String,String>{if !valid_id(&id){return Err("Invalid font ID".into());}let data=read_font(&directory(&app)?.join(format!("{id}.ttf")))?;Ok(base64::engine::general_purpose::STANDARD.encode(data))}
#[tauri::command]
pub async fn import_font(app:tauri::AppHandle)->Result<Option<Font>,String>{
 let Some(file)=app.dialog().file().set_title("Add TTF font").add_filter("TrueType", &["ttf"]).blocking_pick_file() else{return Ok(None);};
 let path=file.into_path().map_err(|e|e.to_string())?;
 let bytes=read_font(&path)?;let id=format!("{:x}",Sha256::digest(&bytes));let font=asset(id.clone(),bytes.clone())?;
 let dir=directory(&app)?;std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;std::fs::write(dir.join(format!("{id}.ttf")),bytes).map_err(|e|e.to_string())?;
 app.emit("fonts-changed",()).map_err(|e|e.to_string())?;Ok(Some(font))
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn rejects_paths_and_invalid_font_data(){assert!(!valid_id("../settings"));assert!(!valid_id(&"z".repeat(64)));assert!(valid_id(&"a".repeat(64)));assert!(asset("a".repeat(64),b"not a font".to_vec()).is_err());}
 #[test]fn installed_font_can_be_imported(){let mut db=fontdb::Database::new();db.load_system_fonts();if let Some(face)=db.faces().next(){let bytes=db.with_face_data(face.id,|data,_|data.to_vec()).unwrap();let id=format!("{:x}",Sha256::digest(&bytes));let font=asset(id.clone(),bytes).unwrap();assert_eq!(font.family,format!("OnPenFont-{id}"));assert!(!font.name.is_empty());};}
}
