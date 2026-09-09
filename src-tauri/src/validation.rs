//! Opt-in native WebView smoke runner. Never compiled into normal releases.
use tauri::{Listener, Manager};

pub fn start(app: &tauri::AppHandle) {
    let Some(report) = std::env::var_os("POINTORY_VALIDATION_REPORT") else { return; };
    // A validation run must never edit the user's normal profile.
    if std::env::var_os("POINTORY_DATA_DIR").is_none() { return; }
    let report = std::path::PathBuf::from(report);
    if !report.is_absolute() { return; }
    if let Err(error) = app.add_capability(r#"{"identifier":"native-validation","windows":["toolbar","settings","overlay","caption-settings","sharing"],"permissions":["core:event:allow-emit"]}"#) {
        eprintln!("Validation capability failed: {error}"); return;
    }
    eprintln!("Pointory native validation starting: {}", report.display());
    app.listen("pointory-validation-progress", |event| eprintln!("Validation: {}", event.payload()));
    let handle = app.clone();
    app.listen("pointory-validation-eval", move |event| {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(event.payload()) else { return; };
        eprintln!("Validation evaluating: {}", value["label"]);
        let (Some(id), Some(label), Some(script)) = (value["id"].as_str(), value["label"].as_str(), value["script"].as_str()) else { return; };
        if !["toolbar", "settings", "overlay", "caption-settings", "sharing"].contains(&label) { return; }
        if let Some(window) = handle.get_webview_window(label) {
            let id = serde_json::to_string(id).unwrap();
            let code = format!("(async()=>{{try{{const data=await (async()=>{{{script}}})();await window.__TAURI__.event.emit('pointory-validation-response',{{id:{id},data}});}}catch(e){{await window.__TAURI__.event.emit('pointory-validation-response',{{id:{id},error:String(e)}});}}}})()");
            let _ = window.eval(&code);
        }
    });
    let handle = app.clone();
    app.listen("pointory-validation-finished", move |event| {
        eprintln!("Validation received final report");
        let mut value = serde_json::from_str::<serde_json::Value>(event.payload()).unwrap_or_default();
        let windows: Vec<_> = ["toolbar", "settings", "overlay"].iter().filter_map(|label| {
            let w = handle.get_webview_window(label)?;
            Some(serde_json::json!({"label":label,"position":w.outer_position().ok(),"size":w.outer_size().ok(),"visible":w.is_visible().ok()}))
        }).collect();
        value["native_windows"] = serde_json::json!(windows);
        if let Some(parent) = report.parent() { let _ = std::fs::create_dir_all(parent); }
        if let Ok(bytes) = serde_json::to_vec_pretty(&value) { let _ = std::fs::write(&report, bytes); }
        if std::env::var_os("POINTORY_VALIDATION_KEEP_OPEN").is_none() { handle.exit(0); }
    });
}

pub fn page_loaded(window: &tauri::WebviewWindow) {
    if std::env::var_os("POINTORY_VALIDATION_REPORT").is_none() || std::env::var_os("POINTORY_DATA_DIR").is_none() { return; }
    eprintln!("Validation injecting into loaded page: {:?}", window.url());
    let screen = std::env::var("POINTORY_VALIDATION_SCREEN").as_deref() == Ok("1");
    let script = format!("window.__POINTORY_VALIDATION_SCREEN={screen};\n{}", include_str!("../../tests/native-smoke.js"));
    if let Err(error) = window.eval(&script) { eprintln!("Validation injection failed: {error}"); }
}
