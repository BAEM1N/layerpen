use std::{io::{BufRead, BufReader, Write}, process::{Child, Command, Stdio}, sync::{Arc, Mutex}, time::Duration};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

#[derive(Default)]
pub struct CaptionState {
    child: Mutex<Option<Arc<Mutex<Child>>>>,
    pub last: Mutex<serde_json::Value>,
}

fn worker(app: &tauri::AppHandle) -> Result<Command,String> {
    let local=app.path().local_data_dir().map_err(|e|e.to_string())?;
    let venv=local.join("OnPen").join("stt-venv").join(if cfg!(windows){"Scripts/python.exe"}else{"bin/python"});
    let python = std::env::var_os("ONPEN_STT_PYTHON").or_else(||std::env::var_os("LAYERPEN_STT_PYTHON")).unwrap_or_else(|| if venv.is_file(){venv.into_os_string()}else{if cfg!(windows){"python".into()}else{"python3".into()}});
    let resource=app.path().resource_dir().map_err(|e|e.to_string())?.join("stt/worker.py");
    let script = std::env::var_os("ONPEN_STT_WORKER").or_else(||std::env::var_os("LAYERPEN_STT_WORKER")).map(std::path::PathBuf::from)
        .unwrap_or_else(|| if resource.is_file(){resource}else{std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../stt/worker.py")});
    let mut cmd = Command::new(python);
    cmd.arg("-u").arg(script).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
    #[cfg(target_os="windows")]
    { use std::os::windows::process::CommandExt; cmd.creation_flags(0x08000000); }
    Ok(cmd)
}

fn publish(app: &tauri::AppHandle, value: serde_json::Value) {
    if let Ok(mut last) = app.state::<CaptionState>().last.lock() { *last = value.clone(); }
    let _ = app.emit("caption", value);
}

pub fn stop(app: &tauri::AppHandle) {
    let state = app.state::<CaptionState>();
    if let Ok(mut slot) = state.child.lock() {
        if let Some(child) = slot.take() {
            if let Ok(mut child) = child.lock() {
                if let Some(input) = child.stdin.as_mut() { let _ = writeln!(input, "stop"); }
                child.stdin.take();
                // A local model can be inside native inference; killing its worker promptly
                // releases audio, credentials, native threads and model memory together.
                #[cfg(target_os="windows")]
                {
                    use std::os::windows::process::CommandExt;
                    // Windows venv python.exe can launch another interpreter process.
                    // Terminate only this worker's process tree, including that interpreter.
                    let _=Command::new("taskkill.exe").args(["/PID",&child.id().to_string(),"/T","/F"])
                        .creation_flags(0x08000000).stdout(Stdio::null()).stderr(Stdio::null()).status();
                }
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
    publish(app, serde_json::json!({"type":"stopped","text":"Stopped"}));
    if let Some(w) = app.get_webview_window("captions") { let _ = w.hide(); }
}

#[tauri::command]
pub async fn caption_open(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("caption-settings") {
        w.show().map_err(|e| e.to_string())?;
        return w.set_focus().map_err(|e| e.to_string());
    }
    WebviewWindowBuilder::new(&app,"caption-settings",WebviewUrl::App("captions.html".into()))
        .data_directory(super::webview_data(&app)?)
        .title("OnPen · Live captions (local preview)").inner_size(660.,740.)
        .build().map_err(|e|e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn caption_snapshot(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    app.state::<CaptionState>().last.lock().map(|v|v.clone()).map_err(|e|e.to_string())
}

#[tauri::command]
pub async fn caption_stop(app: tauri::AppHandle) -> Result<(), String> { stop(&app); Ok(()) }

#[tauri::command]
pub async fn caption_devices(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    query_worker(app,"devices").await
}

#[tauri::command]
pub async fn caption_hardware(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    query_worker(app,"hardware").await
}

async fn query_worker(app: tauri::AppHandle,command: &'static str) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut child = worker(&app)?.spawn().map_err(|_|"STT Python runtime is unavailable. Install the optional STT runtime; see the bundled STT setup guide.".to_string())?;
        if let Some(mut input) = child.stdin.take() { writeln!(input,"{}",serde_json::json!({"command":command})).map_err(|e|e.to_string())?; }
        for _ in 0..300 {
            if child.try_wait().map_err(|e|e.to_string())?.is_some() {
                let out = child.wait_with_output().map_err(|e|e.to_string())?;
                return serde_json::from_slice(&out.stdout).map_err(|_|"Could not list audio devices. Check the STT runtime.".into());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _=child.kill(); let _=child.wait();
        Err("Audio device enumeration timed out.".into())
    }).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub async fn caption_start(app: tauri::AppHandle, config: serde_json::Value) -> Result<(), String> {
    let provider = config["provider"].as_str().ok_or("Choose an STT provider.")?;
    if !["whisper","qwen","openai","gemini","elevenlabs"].contains(&provider) { return Err("Unsupported provider".into()); }
    if !["microphone","system"].contains(&config["source"].as_str().unwrap_or("")) { return Err("Choose an audio source".into()); }
    stop(&app);
    let w = if let Some(w) = app.get_webview_window("captions") { w } else {
        let w=WebviewWindowBuilder::new(&app,"captions",WebviewUrl::App("captions.html?overlay=1".into()))
            .data_directory(super::webview_data(&app)?)
            .title("OnPen · Captions").inner_size(960.,150.).decorations(false)
            .transparent(true).shadow(false).always_on_top(true).skip_taskbar(true)
            .focused(false).visible(false).build().map_err(|e|e.to_string())?;
        w.set_ignore_cursor_events(true).map_err(|e|e.to_string())?;
        w
    };
    let selected = app.state::<super::Shared>().lock().map_err(|e|e.to_string())?.selected().cloned();
    if let Some(m) = selected {
        let width = (m.width as f64 / m.scale - 48.).min(960.).max(200.);
        w.set_size(tauri::LogicalSize::new(width,150.)).map_err(|e|e.to_string())?;
        w.set_position(tauri::PhysicalPosition::new(m.x+((m.width as f64-width*m.scale)/2.) as i32,
            m.y+m.height as i32-(190.*m.scale) as i32)).map_err(|e|e.to_string())?;
    }
    let mut child = worker(&app)?.spawn().map_err(|_|"STT runtime unavailable. Install the optional STT runtime; see the bundled STT setup guide.".to_string())?;
    // Credentials travel through an anonymous pipe, never command-line arguments or settings.
    let mut config = config;
    config.as_object_mut().ok_or("Invalid config")?.remove("wav");
    if let Err(e) = writeln!(child.stdin.as_mut().ok_or("Worker stdin unavailable")?,"{}",config) {
        let _=child.kill(); let _=child.wait(); return Err(e.to_string());
    }
    let output = child.stdout.take().ok_or("Worker stdout unavailable")?;
    let shared=Arc::new(Mutex::new(child));
    *app.state::<CaptionState>().child.lock().map_err(|e|e.to_string())?=Some(shared.clone());
    publish(&app,serde_json::json!({"type":"status","text":"Starting…"}));
    w.show().map_err(|e|e.to_string())?;
    let handle=app.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(output).lines().map_while(Result::ok) {
            let current=handle.state::<CaptionState>().child.lock().ok().and_then(|v|v.clone());
            if !current.as_ref().is_some_and(|c|Arc::ptr_eq(c,&shared)) { break; }
            if line.len() < 32768 {
                if let Ok(event)=serde_json::from_str::<serde_json::Value>(&line) { publish(&handle,event); }
            }
        }
        if let Ok(mut slot)=handle.state::<CaptionState>().child.lock() {
            if slot.as_ref().is_some_and(|c|Arc::ptr_eq(c,&shared)) {
                if let Ok(mut child)=shared.lock() { let _=child.wait(); }
                *slot=None;
                publish(&handle,serde_json::json!({"type":"stopped","text":"Stopped"}));
            }
        };
    });
    Ok(())
}
