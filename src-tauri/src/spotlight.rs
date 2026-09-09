use std::sync::{Arc,atomic::{AtomicBool,AtomicU64,Ordering}};
use tauri::{Emitter,Manager,PhysicalPosition,PhysicalSize,WebviewUrl,WebviewWindowBuilder};
use base64::Engine;
type Result<T> = std::result::Result<T,String>;
#[derive(Default)]pub struct SpotlightState { pub enabled:Arc<AtomicBool>, generation:Arc<AtomicU64> }
pub fn stop(app:&tauri::AppHandle){let state=app.state::<SpotlightState>();state.enabled.store(false,Ordering::SeqCst);state.generation.fetch_add(1,Ordering::SeqCst);if let Some(w)=app.get_webview_window("spotlight"){let _=w.hide();}}
#[tauri::command]
pub fn spotlight_status(app:tauri::AppHandle)->bool{app.state::<SpotlightState>().enabled.load(Ordering::SeqCst)}
#[tauri::command]
pub async fn spotlight_toggle(app:tauri::AppHandle)->Result<()> {
 if spotlight_status(app.clone()){stop(&app);return Ok(());}
 let display=app.state::<super::Shared>().lock().map_err(|e|e.to_string())?.selected().ok_or("Select a monitor")?.clone();
 let window=if let Some(w)=app.get_webview_window("spotlight"){w}else{WebviewWindowBuilder::new(&app,"spotlight",WebviewUrl::App("spotlight.html".into())).title("Pointory · Spotlight").transparent(true).decorations(false).shadow(false).always_on_top(true).skip_taskbar(true).focused(false).visible(false).build().map_err(|e|e.to_string())?};
 window.set_ignore_cursor_events(true).map_err(|e|e.to_string())?;
 #[cfg(target_os="windows")]
 window.set_content_protected(true).map_err(|e|e.to_string())?;
 window.set_position(PhysicalPosition::new(display.x,display.y)).map_err(|e|e.to_string())?;window.set_size(PhysicalSize::new(display.width,display.height)).map_err(|e|e.to_string())?;
 window.show().map_err(|e|e.to_string())?;
 let state=app.state::<SpotlightState>();let flag=state.enabled.clone();let generations=state.generation.clone();let generation=generations.fetch_add(1,Ordering::SeqCst)+1;flag.store(true,Ordering::SeqCst);
 std::thread::spawn(move||{
  let mut last=std::time::Instant::now()-std::time::Duration::from_secs(1);
  #[cfg(target_os="windows")]
  let monitor=xcap::Monitor::all().ok().and_then(|ms|ms.into_iter().find(|m|m.x().ok()==Some(display.x)&&m.y().ok()==Some(display.y)));
  while flag.load(Ordering::SeqCst)&&generations.load(Ordering::SeqCst)==generation {
   let prefs={let s=app.state::<super::Shared>();let Ok(s)=s.lock()else{break;};if s.selected().is_none_or(|m|m.id!=display.id){break;}s.prefs.clone()};
   let Ok(cursor)=app.cursor_position()else{break;};let x=cursor.x-display.x as f64;let y=cursor.y-display.y as f64;
   let inside=x>=0.&&y>=0.&&x<display.width as f64&&y<display.height as f64;
   let scale=if cfg!(target_os="windows"){prefs.spotlight_scale}else{1.};
   let mut frame=serde_json::json!({"x":x/display.scale,"y":y/display.scale,"radius":prefs.spotlight_radius,"dim":prefs.spotlight_dim,"inside":inside,"scale":scale});
   if inside&&scale>1.&&last.elapsed()>std::time::Duration::from_millis(125){
    #[cfg(target_os="windows")]
    if let Some(monitor)=&monitor {match monitor.capture_image(){Ok(image)=>{
     let (left,top,size)=crop(x,y,prefs.spotlight_radius*display.scale/scale,image.width(),image.height());
     let image=xcap::image::imageops::crop_imm(&image,left,top,size,size).to_image();let image=xcap::image::DynamicImage::ImageRgba8(image).to_rgb8();let mut bytes=vec![];
     if xcap::image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes,85).encode_image(&image).is_ok(){frame["image"]=format!("data:image/jpeg;base64,{}",base64::engine::general_purpose::STANDARD.encode(bytes)).into();}
    },Err(e)=>{let _=app.emit("capture-error",format!("Spotlight magnifier: {e}"));break;}}}
    last=std::time::Instant::now();
   }
   let _=app.emit_to("spotlight","spotlight-frame",frame);
   std::thread::sleep(std::time::Duration::from_millis(25));
  }
  if generations.load(Ordering::SeqCst)==generation{stop(&app);}
 });Ok(())
}
fn crop(x:f64,y:f64,r:f64,w:u32,h:u32)->(u32,u32,u32){let size=(r*2.).round().max(1.).min(w.min(h) as f64) as u32;((x-size as f64/2.).clamp(0.,(w-size) as f64) as u32,(y-size as f64/2.).clamp(0.,(h-size) as f64) as u32,size)}

/// The protected spotlight window is excluded from Windows capture. Reapply its effect to LAN frames.
pub fn composite(app:&tauri::AppHandle,display:&super::Display,image:&mut xcap::image::RgbImage){
 if !cfg!(target_os="windows"){return;}
 if !app.state::<SpotlightState>().enabled.load(Ordering::SeqCst){return;}
 let Ok(cursor)=app.cursor_position()else{return;};let Ok(s)=app.state::<super::Shared>().lock().map(|s|s.prefs.clone())else{return;};
 let x=(cursor.x-display.x as f64)*image.width() as f64/display.width as f64;let y=(cursor.y-display.y as f64)*image.height() as f64/display.height as f64;
 if x<0.||y<0.||x>=image.width() as f64||y>=image.height() as f64{return;}
 let radius=s.spotlight_radius*display.scale*image.width() as f64/display.width as f64;let original=image.clone();let scale=if cfg!(target_os="windows"){s.spotlight_scale}else{1.};
 for (px,py,pixel) in image.enumerate_pixels_mut(){let dx=px as f64-x;let dy=py as f64-y;if dx*dx+dy*dy>radius*radius{for c in &mut pixel.0{*c=(*c as f64*(1.-s.spotlight_dim)) as u8;}}else if scale>1.{let sx=(x+dx/scale).clamp(0.,(original.width()-1) as f64) as u32;let sy=(y+dy/scale).clamp(0.,(original.height()-1) as f64) as u32;*pixel=*original.get_pixel(sx,sy);}}
}
#[cfg(test)]mod tests{use super::*;#[test]fn lens_crop_stays_inside_monitor(){assert_eq!(crop(0.,0.,100.,1920,1080),(0,0,200));assert_eq!(crop(1919.,1079.,100.,1920,1080),(1720,880,200));assert_eq!(crop(1.,1.,500.,80,60),(0,0,60));}}
