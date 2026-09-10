use std::{io::{BufRead, BufReader, Read, Write}, path::PathBuf, process::{Child, Command, Stdio}, sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

type Worker = Arc<Mutex<Child>>;
const MAX_MODEL_EVENT: usize = 32768;
const RUNTIME_MISSING: &str = "The speech recognition runtime is unavailable. Install the optional runtime to prepare local speech recognition.";

#[derive(Default)]
pub struct CaptionState {
    child: Mutex<Option<Worker>>,
    pub last: Mutex<serde_json::Value>,
    // Never hold this gate while invoking native window APIs. It only serializes
    // process lifecycle changes, so starting captions and downloading cannot race.
    process_gate: Mutex<()>,
    shutting_down: AtomicBool,
}

#[derive(Default)]
pub struct ModelState {
    child: Mutex<Option<Worker>>,
    last: Mutex<serde_json::Value>,
}

fn python_override(mut read: impl FnMut(&str) -> Option<std::ffi::OsString>) -> Option<std::ffi::OsString> {
    ["POINTORY_STT_PYTHON", "ONPEN_STT_PYTHON", "LAYERPEN_STT_PYTHON"].iter().find_map(|name|read(name))
}

fn python(app: &tauri::AppHandle) -> Result<std::ffi::OsString, String> {
    let local=app.path().local_data_dir().map_err(|e|e.to_string())?;
    let venv=["Pointory", "OnPen", "LayerPen", "MonitorInk"].iter()
        .map(|name| local.join(name).join("stt-venv").join(if cfg!(windows){"Scripts/python.exe"}else{"bin/python"}))
        .find(|path| path.is_file());
    Ok(python_override(|name|std::env::var_os(name))
        .unwrap_or_else(|| venv.map(|path| path.into_os_string()).unwrap_or_else(|| if cfg!(windows){"python".into()}else{"python3".into()})))
}

fn script(app: &tauri::AppHandle, name: &str) -> Result<PathBuf, String> {
    let worker_override = std::env::var_os("POINTORY_STT_WORKER").or_else(||std::env::var_os("ONPEN_STT_WORKER")).or_else(||std::env::var_os("LAYERPEN_STT_WORKER")).map(PathBuf::from);
    let explicit = match name {
        "model_manager.py" => std::env::var_os("POINTORY_STT_MODEL_MANAGER").map(PathBuf::from),
        "runtime_setup.py" => std::env::var_os("POINTORY_STT_RUNTIME_SETUP").map(PathBuf::from),
        _ => worker_override.clone(),
    };
    if let Some(path) = explicit { return Ok(path); }
    if let Some(path) = worker_override.and_then(|p|p.parent().map(|parent|parent.join(name))) {
        return Ok(path);
    }
    let resource = app.path().resource_dir().map_err(|e|e.to_string())?.join("stt").join(name);
    Ok(if resource.is_file() { resource } else { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../stt").join(name) })
}

fn script_worker(app: &tauri::AppHandle, name: &str) -> Result<Command, String> {
    let mut cmd = Command::new(python(app)?);
    cmd.arg("-u").arg(script(app, name)?).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
    #[cfg(target_os="windows")]
    { use std::os::windows::process::CommandExt; cmd.creation_flags(0x08000000); }
    #[cfg(unix)]
    { use std::os::unix::process::CommandExt; cmd.process_group(0); }
    Ok(cmd)
}

fn worker(app: &tauri::AppHandle) -> Result<Command,String> { script_worker(app, "worker.py") }

fn terminate(child: &mut Child) {
    child.stdin.take();
    if child.try_wait().is_ok_and(|status|status.is_some()) { return; }
    #[cfg(target_os="windows")]
    {
        use std::os::windows::process::CommandExt;
        // A Windows venv interpreter and runtime setup's pip may be descendants.
        // Terminate this owned process tree, never other Python processes.
        let _=Command::new("taskkill.exe").args(["/PID",&child.id().to_string(),"/T","/F"])
            .creation_flags(0x08000000).stdout(Stdio::null()).stderr(Stdio::null()).status();
    }
    #[cfg(unix)]
    {
        // script_worker creates a separate process group. Include pip's children
        // on macOS/Linux without ever signalling Pointory's own process group.
        let _ = Command::new("/bin/kill").args(["-TERM", "--", &format!("-{}", child.id())])
            .stdout(Stdio::null()).stderr(Stdio::null()).status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn stop_child(slot: &Mutex<Option<Worker>>, request_stop: bool) {
    // Detach before waiting so completion and cancellation never wait while
    // holding the child slot. A stale reader cannot clear a replacement worker.
    let child = slot.lock().ok().and_then(|mut slot|slot.take());
    if let Some(child) = child {
        if let Ok(mut child) = child.lock() {
            if request_stop {
                if let Some(input) = child.stdin.as_mut() { let _ = writeln!(input, "stop"); }
            }
            terminate(&mut child);
        }
    }
}

fn is_current<T>(slot: &Option<Arc<T>>, child: &Arc<T>) -> bool {
    slot.as_ref().is_some_and(|current|Arc::ptr_eq(current, child))
}

fn wait_current(slot: &Mutex<Option<Worker>>, child: &Worker) -> bool {
    loop {
        if !slot.lock().is_ok_and(|slot|is_current(&slot, child)) { return false; }
        // Only try_wait owns Child's mutex, for a nonblocking check. Holding it
        // across wait() would prevent cancellation of a worker that closed stdout.
        match child.lock().map(|mut child|child.try_wait()) {
            Ok(Ok(None)) => std::thread::sleep(Duration::from_millis(50)),
            _ => return true,
        }
    }
}

fn publish(app: &tauri::AppHandle, value: serde_json::Value) {
    if let Ok(mut last) = app.state::<CaptionState>().last.lock() { *last = value.clone(); }
    let _ = app.emit("caption", value);
}

pub fn stop(app: &tauri::AppHandle) {
    let state = app.state::<CaptionState>();
    if let Ok(_gate) = state.process_gate.lock() {
        stop_child(&state.child, true);
    }
    publish(app, serde_json::json!({"type":"stopped","text":"Stopped"}));
    if let Some(w) = app.get_webview_window("captions") { let _ = w.hide(); }
}

pub fn shutdown(app: &tauri::AppHandle) {
    app.state::<CaptionState>().shutting_down.store(true, Ordering::Release);
    stop(app);
    let state = app.state::<CaptionState>();
    if let Ok(_gate) = state.process_gate.lock() {
        stop_child(&app.state::<ModelState>().child, false);
    };
}

#[tauri::command]
pub async fn caption_open(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("caption-settings") {
        super::macos::set_level(&w)?;
        w.show().map_err(|e| e.to_string())?;
        return w.set_focus().map_err(|e| e.to_string());
    }
    let window = WebviewWindowBuilder::new(&app,"caption-settings",WebviewUrl::App("captions.html".into()))
        .data_directory(super::webview_data(&app)?)
        .title("Pointory · Live captions (local preview)").inner_size(660.,740.)
        .build().map_err(|e|e.to_string())?;
    super::macos::set_level(&window)?;
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
        if app.state::<CaptionState>().shutting_down.load(Ordering::Acquire) { return Err("Pointory is shutting down.".into()); }
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

fn accelerator_valid(value: &str) -> bool {
    if ["auto", "cpu", "cuda"].contains(&value) { return true; }
    let Some(device) = value.strip_prefix("openvino:") else { return false; };
    let (name, index) = device.split_once('.').map_or((device, None), |(name,index)|(name,Some(index)));
    ["CPU", "GPU", "NPU"].contains(&name)
        && index.is_none_or(|index|!index.is_empty() && index.len() <= 3 && index.bytes().all(|b|b.is_ascii_digit()))
}

fn model_config(config: &serde_json::Value) -> Result<serde_json::Value, String> {
    let provider = config["provider"].as_str().ok_or("Choose a local speech recognition provider.")?;
    if !["whisper", "qwen"].contains(&provider) { return Err("Model downloads are available for local providers only.".into()); }
    let model = config["model"].as_str().unwrap_or("").trim();
    let model = if model.is_empty() { if provider == "whisper" { "base" } else { "Qwen/Qwen3-ASR-0.6B" } } else { model };
    // Custom local model folders remain supported. Python resolves the official
    // download allowlist; only bounded identifiers and paths reach this IPC.
    if model.len() > 1024 || model.chars().any(char::is_control) { return Err("The model name or folder is invalid.".into()); }
    let accelerator = config["accelerator"].as_str().unwrap_or("auto");
    if !accelerator_valid(accelerator) { return Err("Choose an available speech recognition accelerator.".into()); }
    let mut request = serde_json::json!({"provider":provider,"model":model,"accelerator":accelerator});
    if accelerator == "auto" {
        if let Some(resolved) = config["resolvedAccelerator"].as_str() {
            if resolved == "auto" || !accelerator_valid(resolved) { return Err("The resolved speech recognition accelerator is invalid.".into()); }
            request["resolvedAccelerator"] = resolved.into();
        }
    }
    Ok(request)
}

fn model_engine(config: &serde_json::Value) -> &'static str {
    if config["provider"] == "qwen" { return "qwen"; }
    let accelerator = if config["accelerator"] == "auto" { config["resolvedAccelerator"].as_str().unwrap_or("cpu") } else { config["accelerator"].as_str().unwrap_or("cpu") };
    if accelerator.starts_with("openvino:") { "openvino" } else { "faster-whisper" }
}

fn model_event(config: &serde_json::Value, kind: &str, code: &str, text: &str) -> serde_json::Value {
    serde_json::json!({"type":kind,"provider":config["provider"],"model":config["model"],
        "engine":model_engine(config),"accelerator":config["accelerator"],"code":code,"text":text})
}

fn publish_model(app: &tauri::AppHandle, value: serde_json::Value) {
    if let Ok(mut last) = app.state::<ModelState>().last.lock() { *last = value.clone(); }
    let _ = app.emit("caption-model", value);
}

fn read_bounded_line(reader: &mut impl BufRead) -> std::io::Result<Option<String>> {
    let mut line = String::new();
    let size = reader.take((MAX_MODEL_EVENT + 1) as u64).read_line(&mut line)?;
    if size > MAX_MODEL_EVENT { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Model event exceeded its limit")); }
    Ok(if size == 0 { None } else { Some(line) })
}

fn parse_model_event(line: &str) -> Option<serde_json::Value> {
    let event: serde_json::Value = serde_json::from_str(line).ok()?;
    let kind = event["type"].as_str()?;
    ["model_status", "model_progress", "model_ready", "model_error", "runtime_ready"].contains(&kind).then_some(event)
}

fn model_terminal(event: &serde_json::Value) -> bool {
    matches!(event["type"].as_str(), Some("model_ready" | "model_error" | "runtime_ready"))
}

fn write_request(child: &mut Child, request: &serde_json::Value) -> Result<(), String> {
    let result = child.stdin.take().ok_or_else(||"The speech recognition worker could not receive its configuration.".to_string())
        .and_then(|mut input| writeln!(input,"{request}").map_err(|_|"The speech recognition worker could not receive its configuration.".into()));
    if result.is_err() { terminate(child); }
    result
}

#[tauri::command]
pub async fn caption_model_status(app: tauri::AppHandle, config: serde_json::Value) -> Result<serde_json::Value, String> {
    let config = model_config(&config)?;
    tauri::async_runtime::spawn_blocking(move || {
        if app.state::<CaptionState>().shutting_down.load(Ordering::Acquire) { return Err("Pointory is shutting down.".into()); }
        let mut request = config.clone();
        request["command"] = "status".into();
        let mut command = script_worker(&app, "model_manager.py")?;
        // A status request must never connect to a model repository, including
        // from imported runtime libraries. Only Download enables network access.
        command.env("HF_HUB_OFFLINE", "1").env("HF_HUB_DISABLE_TELEMETRY", "1");
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(_) => return Ok(model_event(&config,"model_error","runtime_missing",RUNTIME_MISSING)),
        };
        if write_request(&mut child, &request).is_err() {
            return Ok(model_event(&config,"model_error","runtime_missing",RUNTIME_MISSING));
        }
        let Some(output) = child.stdout.take() else {
            terminate(&mut child);
            return Ok(model_event(&config,"model_error","runtime_missing",RUNTIME_MISSING));
        };
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let mut reader = BufReader::new(output);
            let result = read_bounded_line(&mut reader).ok().flatten().and_then(|line|parse_model_event(&line));
            let _ = sender.send(result);
        });
        let deadline = Instant::now() + Duration::from_secs(30);
        let result = receiver.recv_timeout(Duration::from_secs(30)).ok().flatten();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if let Some(result) = result {
                        // Structured errors are useful even when the helper exits 1.
                        if status.success() || result["type"] == "model_error" { return Ok(result); }
                    }
                    return Ok(model_event(&config,"model_error","runtime_missing",RUNTIME_MISSING));
                }
                Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(50)),
                _ => {
                    terminate(&mut child);
                    return Ok(model_event(&config,"model_error","status_timeout","Checking the local model timed out. Check the speech recognition runtime and try again."));
                }
            }
        }
    }).await.map_err(|_|"Checking the local model failed.".to_string())?
}

#[tauri::command]
pub async fn caption_model_snapshot(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let state = app.state::<ModelState>();
    // Include active separately because a terminal event can arrive just before
    // the helper exits. Controls must wait for process completion, too.
    let active = state.child.lock().map_err(|_|"Model state is unavailable.")?.is_some();
    let mut last = state.last.lock().map_err(|_|"Model state is unavailable.")?.clone();
    if !last.is_object() { last = serde_json::json!({"type":"model_status","status":"idle"}); }
    last["active"] = active.into();
    Ok(last)
}

fn start_model_job(app: &tauri::AppHandle, config: serde_json::Value, setup: bool) -> Result<serde_json::Value, String> {
    let config = model_config(&config)?;
    let state = app.state::<CaptionState>();
    let _gate = state.process_gate.lock().map_err(|_|"Speech recognition state is unavailable.")?;
    if state.shutting_down.load(Ordering::Acquire) { return Err("Pointory is shutting down.".into()); }
    if state.child.lock().map_err(|_|"Speech recognition state is unavailable.")?.is_some() {
        return Err("Stop live captions before preparing a local model.".into());
    }
    let models = app.state::<ModelState>();
    if models.child.lock().map_err(|_|"Model state is unavailable.")?.is_some() {
        return Err("A model download or runtime setup is already running.".into());
    }
    if setup && python_override(|name|std::env::var_os(name)).is_some() {
        let event = model_event(&config,"model_error","runtime_override","A custom speech Python runtime is configured. Install dependencies there, or remove the speech Python override to use Pointory's managed runtime.");
        publish_model(app,event.clone());
        return Ok(event);
    }
    let (name, request) = if setup {
        let directory = app.path().local_data_dir().map_err(|_|"The speech recognition runtime folder is unavailable.")?.join("Pointory/stt-venv");
        ("runtime_setup.py", serde_json::json!({"engine":model_engine(&config),"runtimeDir":directory}))
    } else {
        let mut request = config.clone();
        request["command"] = "download".into();
        ("model_manager.py", request)
    };
    let mut command = script_worker(app, name)?;
    command.env("HF_HUB_DISABLE_TELEMETRY", "1");
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => {
            let event = model_event(&config,"model_error","runtime_missing",RUNTIME_MISSING);
            publish_model(app, event.clone());
            return Ok(event);
        }
    };
    if write_request(&mut child, &request).is_err() {
        let event = model_event(&config,"model_error","runtime_missing",RUNTIME_MISSING);
        publish_model(app, event.clone());
        return Ok(event);
    }
    let Some(output) = child.stdout.take() else {
        terminate(&mut child);
        return Err("The model worker could not start its progress stream.".into());
    };
    let shared = Arc::new(Mutex::new(child));
    *models.child.lock().map_err(|_|"Model state is unavailable.")? = Some(shared.clone());
    let mut event = model_event(&config,"model_progress","",if setup { "Preparing the speech recognition runtime…" } else { "Preparing model download…" });
    event["phase"] = if setup { "runtime_setup" } else { "starting" }.into();
    event["active"] = true.into();
    event["operation"] = if setup { "runtime" } else { "download" }.into();
    publish_model(app, event.clone());
    let handle = app.clone();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(output);
        let mut terminal = None;
        let mut invalid = false;
        loop {
            let line = match read_bounded_line(&mut reader) {
                Ok(Some(line)) => line,
                Ok(None) => break,
                Err(_) => { invalid = true; break; }
            };
            let Some(mut event) = parse_model_event(&line) else { invalid = true; break; };
            let models = handle.state::<ModelState>();
            let Ok(slot) = models.child.lock() else { return; };
            if !is_current(&slot, &shared) { return; }
            // Match progress from runtime_setup to the model selection that began
            // this job. No API key or full caption configuration is retained.
            event["provider"] = config["provider"].clone();
            event["model"] = config["model"].clone();
            event["engine"] = model_engine(&config).into();
            event["operation"] = if setup { "runtime" } else { "download" }.into();
            if model_terminal(&event) { terminal = Some(event); }
            else {
                event["active"] = true.into();
                publish_model(&handle,event);
            }
        }
        let models = handle.state::<ModelState>();
        if invalid {
            // A malformed or oversized event must not strand a live process.
            if let Ok(slot) = models.child.lock() {
                if !is_current(&slot, &shared) { return; }
            }
            if let Ok(mut child) = shared.lock() { terminate(&mut child); }
        }
        if !wait_current(&models.child, &shared) { return; }
        if let Ok(mut slot) = models.child.lock() {
            if is_current(&slot, &shared) {
                *slot = None;
                let mut event = terminal.unwrap_or_else(||model_event(&config,"model_error",if invalid { "worker_protocol" } else { "runtime_missing" },if invalid { "The model worker returned invalid progress. Try again or reinstall the speech recognition runtime." } else { RUNTIME_MISSING }));
                event["active"] = false.into();
                event["operation"] = if setup { "runtime" } else { "download" }.into();
                publish_model(&handle,event);
            }
        };
    });
    Ok(event)
}

#[tauri::command]
pub async fn caption_model_download(app: tauri::AppHandle, config: serde_json::Value) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move ||start_model_job(&app,config,false)).await.map_err(|_|"The model download could not start.".to_string())?
}

#[tauri::command]
pub async fn caption_runtime_setup(app: tauri::AppHandle, config: serde_json::Value) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move ||start_model_job(&app,config,true)).await.map_err(|_|"The runtime setup could not start.".to_string())?
}

#[tauri::command]
pub async fn caption_model_cancel(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let captions = app.state::<CaptionState>();
        let _gate = captions.process_gate.lock().map_err(|_|"Speech recognition state is unavailable.")?;
        let models = app.state::<ModelState>();
        let active = models.child.lock().map_err(|_|"Model state is unavailable.")?.is_some();
        stop_child(&models.child,false);
        let mut event = models.last.lock().map_err(|_|"Model state is unavailable.")?.clone();
        if !event.is_object() { event = serde_json::json!({}); }
        if active {
            event["type"] = "model_status".into();
            event["status"] = "cancelled".into();
            event["code"] = "cancelled".into();
            event["text"] = "Preparation cancelled. Downloaded files are kept so you can try again.".into();
            event["ready"] = false.into();
        }
        event["active"] = false.into();
        publish_model(&app,event.clone());
        Ok(event)
    }).await.map_err(|_|"The model worker could not be cancelled.".to_string())?
}

#[tauri::command]
pub async fn caption_start(app: tauri::AppHandle, config: serde_json::Value) -> Result<(), String> {
    let provider = config["provider"].as_str().ok_or("Choose an STT provider.")?;
    if !["whisper","qwen","openai","gemini","elevenlabs"].contains(&provider) { return Err("Unsupported provider".into()); }
    if !["microphone","system"].contains(&config["source"].as_str().unwrap_or("")) { return Err("Choose an audio source".into()); }
    if app.state::<CaptionState>().shutting_down.load(Ordering::Acquire) { return Err("Pointory is shutting down.".into()); }
    if app.state::<ModelState>().child.lock().map_err(|_|"Model state is unavailable.")?.is_some() {
        return Err("Wait for model preparation to finish or cancel it before starting live captions.".into());
    }
    stop(&app);
    let w = if let Some(w) = app.get_webview_window("captions") { w } else {
        let w=WebviewWindowBuilder::new(&app,"captions",WebviewUrl::App("captions.html?overlay=1".into()))
            .data_directory(super::webview_data(&app)?)
            .title("Pointory · Captions").inner_size(960.,150.).decorations(false)
            .transparent(true).shadow(false).always_on_top(true).skip_taskbar(true)
            .focused(false).visible(false).build().map_err(|e|e.to_string())?;
        w.set_ignore_cursor_events(true).map_err(|e|e.to_string())?;
        w
    };
    let selected = app.state::<super::Shared>().lock().map_err(|e|e.to_string())?.selected().cloned();
    super::macos::set_level(&w)?;
    if let Some(m) = selected {
        let width = (m.width as f64 / m.scale - 48.).min(960.).max(200.);
        w.set_size(tauri::LogicalSize::new(width,150.)).map_err(|e|e.to_string())?;
        w.set_position(tauri::PhysicalPosition::new(m.x+((m.width as f64-width*m.scale)/2.) as i32,
            m.y+m.height as i32-(190.*m.scale) as i32)).map_err(|e|e.to_string())?;
    }
    let (shared, output) = {
        let captions = app.state::<CaptionState>();
        let _gate = captions.process_gate.lock().map_err(|_|"Speech recognition state is unavailable.")?;
        if captions.shutting_down.load(Ordering::Acquire) { return Err("Pointory is shutting down.".into()); }
        if app.state::<ModelState>().child.lock().map_err(|_|"Model state is unavailable.")?.is_some() {
            return Err("Wait for model preparation to finish or cancel it before starting live captions.".into());
        }
        stop_child(&captions.child, true);
        let mut child = worker(&app)?.spawn().map_err(|_|"STT runtime unavailable. Install the optional STT runtime; see the bundled STT setup guide.".to_string())?;
        // Credentials travel through an anonymous pipe, never command-line arguments or settings.
        let mut config = config;
        config.as_object_mut().ok_or("Invalid config")?.remove("wav");
        if child.stdin.as_mut().is_none_or(|input|writeln!(input,"{}",config).is_err()) {
            terminate(&mut child);
            return Err("The caption worker could not receive its configuration.".into());
        }
        let Some(output) = child.stdout.take() else {
            terminate(&mut child);
            return Err("The caption worker could not start its progress stream.".into());
        };
        let shared = Arc::new(Mutex::new(child));
        *captions.child.lock().map_err(|_|"Speech recognition state is unavailable.")? = Some(shared.clone());
        (shared, output)
    };
    publish(&app,serde_json::json!({"type":"status","text":"Starting…"}));
    if let Err(error) = w.show() { stop(&app); return Err(error.to_string()); }
    let handle=app.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(output).lines().map_while(Result::ok) {
            let captions = handle.state::<CaptionState>();
            let Ok(slot) = captions.child.lock() else { return; };
            if !is_current(&slot, &shared) { return; }
            if line.len() < 32768 {
                if let Ok(event)=serde_json::from_str::<serde_json::Value>(&line) { publish(&handle,event); }
            }
        }
        let captions = handle.state::<CaptionState>();
        if !wait_current(&captions.child, &shared) { return; }
        if let Ok(mut slot)=captions.child.lock() {
            if is_current(&slot,&shared) {
                *slot=None;
                publish(&handle,serde_json::json!({"type":"stopped","text":"Stopped"}));
            }
        };
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_python_overrides_are_detected_in_precedence_order() {
        let values = ["primary-python", "onpen-python", "layerpen-python"];
        for first in 0..3 {
            let selected = python_override(|name| {
                let index = ["POINTORY_STT_PYTHON", "ONPEN_STT_PYTHON", "LAYERPEN_STT_PYTHON"].iter().position(|key|*key == name).unwrap();
                (index >= first).then(||values[index].into())
            });
            assert_eq!(selected, Some(std::ffi::OsString::from(values[first])));
        }
        assert!(python_override(|_|None).is_none());
    }

    #[test]
    fn model_requests_exclude_credentials_and_untrusted_runtime_paths() {
        let config = model_config(&serde_json::json!({"provider":"whisper","model":"base","accelerator":"auto",
            "resolvedAccelerator":"cuda","api_key":"private-test-key","source":"microphone","device":"private-device",
            "runtimeDir":"C:/somewhere-else","engine":"qwen"})).unwrap();
        assert_eq!(config, serde_json::json!({"provider":"whisper","model":"base","accelerator":"auto","resolvedAccelerator":"cuda"}));
        assert_eq!(model_engine(&config), "faster-whisper");
    }

    #[test]
    fn model_validation_preserves_local_folders_but_rejects_invalid_ids() {
        let local = model_config(&serde_json::json!({"provider":"whisper","model":"C:\\Models\\Local Whisper","accelerator":"cpu"})).unwrap();
        assert_eq!(local["model"], "C:\\Models\\Local Whisper");
        for invalid in [serde_json::json!({"provider":"openai"}),
            serde_json::json!({"provider":"whisper","model":"base\nsecret"}),
            serde_json::json!({"provider":"whisper","model":"x".repeat(1025)}),
            serde_json::json!({"provider":"whisper","accelerator":"openvino:NPU; command"}),
            serde_json::json!({"provider":"whisper","accelerator":"auto","resolvedAccelerator":"auto"})] {
            assert!(model_config(&invalid).is_err());
        }
        assert_eq!(model_config(&serde_json::json!({"provider":"qwen"})).unwrap()["model"], "Qwen/Qwen3-ASR-0.6B");
    }

    #[test]
    fn runtime_engine_tracks_effective_acceleration() {
        for (provider, requested, resolved, expected) in [
            ("whisper", "auto", "openvino:GPU.0", "openvino"),
            ("whisper", "openvino:NPU", "cpu", "openvino"),
            ("whisper", "auto", "cuda", "faster-whisper"),
            ("whisper", "cpu", "cuda", "faster-whisper"),
            ("qwen", "cpu", "cpu", "qwen"),
        ] {
            let config = model_config(&serde_json::json!({"provider":provider,"accelerator":requested,"resolvedAccelerator":resolved})).unwrap();
            assert_eq!(model_engine(&config), expected);
            if requested != "auto" { assert!(config.get("resolvedAccelerator").is_none()); }
        }
    }

    #[test]
    fn progress_stream_is_bounded_and_requires_known_event_types() {
        let input = b"{\"type\":\"model_progress\"}\n{\"type\":\"runtime_ready\"}\n";
        let mut reader = BufReader::new(&input[..]);
        let progress = parse_model_event(&read_bounded_line(&mut reader).unwrap().unwrap()).unwrap();
        assert!(!model_terminal(&progress));
        let ready = parse_model_event(&read_bounded_line(&mut reader).unwrap().unwrap()).unwrap();
        assert!(model_terminal(&ready));
        assert!(read_bounded_line(&mut reader).unwrap().is_none());
        let excessive = vec![b'x'; MAX_MODEL_EVENT + 1];
        assert!(read_bounded_line(&mut BufReader::new(&excessive[..])).is_err());
        assert!(parse_model_event("{\"type\":\"caption\",\"text\":\"unexpected\"}").is_none());
        assert!(parse_model_event("not-json").is_none());
    }

    #[test]
    fn stale_completion_cannot_match_a_replacement() {
        let old = Arc::new(());
        let replacement = Arc::new(());
        assert!(is_current(&Some(old.clone()), &old));
        assert!(!is_current(&Some(replacement), &old));
        assert!(!is_current(&None, &old));
    }

    #[test]
    fn worker_fixture() {
        if std::env::var("POINTORY_CANCEL_TEST_CHILD").as_deref() == Ok("1") {
            std::thread::sleep(Duration::from_secs(30));
        }
    }

    #[test]
    fn cancellation_releases_a_completion_wait_without_holding_child_lock() {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", "captions::tests::worker_fixture", "--nocapture"])
            .env("POINTORY_CANCEL_TEST_CHILD", "1").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        #[cfg(target_os="windows")]
        { use std::os::windows::process::CommandExt; command.creation_flags(0x08000000); }
        #[cfg(unix)]
        { use std::os::unix::process::CommandExt; command.process_group(0); }
        let child = Arc::new(Mutex::new(command.spawn().unwrap()));
        let slot = Arc::new(Mutex::new(Some(child.clone())));
        let waiting = slot.clone();
        let waiter = std::thread::spawn(move ||wait_current(&waiting, &child));
        std::thread::sleep(Duration::from_millis(100));
        let started = Instant::now();
        stop_child(&slot, false);
        assert!(!waiter.join().unwrap(), "Cancellation must detach the current worker");
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(slot.lock().unwrap().is_none());
    }
}
