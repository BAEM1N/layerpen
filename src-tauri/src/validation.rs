//! Opt-in native WebView smoke runner. Never compiled into normal releases.
use std::{path::{Path, PathBuf}, sync::{Arc, Mutex}, time::Duration};
use serde_json::{json, Value};
use tauri::{Emitter, Listener, Manager};

fn paths() -> Option<(PathBuf, PathBuf)> {
    let report = PathBuf::from(std::env::var_os("POINTORY_VALIDATION_REPORT")?);
    // A validation run must never edit the user's normal profile.
    let profile = PathBuf::from(std::env::var_os("POINTORY_DATA_DIR")?);
    (report.is_absolute() && profile.is_absolute()).then_some((report, profile))
}

fn read_json(path: &Path) -> Result<Value, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()))
}

// JavaScript emits whole-valued widths as integers; Rust stores them as f64.
// Settings numbers are bounded, so compare their numeric values without rounding.
fn settings_json_equal(expected: &Value, actual: &Value) -> bool {
    match (expected, actual) {
        (Value::Number(left), Value::Number(right)) => left.as_f64().zip(right.as_f64())
            .is_some_and(|(left, right)| left == right),
        (Value::Array(left), Value::Array(right)) => left.len() == right.len()
            && left.iter().zip(right).all(|(left, right)| settings_json_equal(left, right)),
        (Value::Object(left), Value::Object(right)) => left.len() == right.len()
            && left.iter().all(|(key, left)| right.get(key).is_some_and(|right| settings_json_equal(left, right))),
        _ => expected == actual,
    }
}

fn persistence_check(name: &str, expected: &Value, actual: &Value, fields: Option<&[&str]>) -> Value {
    let valid = expected.is_object() && actual.is_object();
    let equal = match fields {
        Some(fields) => fields.iter().all(|field| !expected[*field].is_null() && settings_json_equal(&expected[*field], &actual[*field])),
        None => settings_json_equal(expected, actual),
    };
    if valid && equal {
        json!({"name":name,"status":"passed","detail":{"text_font":actual["text_font"],"settings_font_size":actual["settings_font_size"],"pen_width":actual["pen_width"]}})
    } else {
        json!({"name":name,"status":"failed","error":"Saved preferences differ from native session preferences","expected":expected,"actual":actual})
    }
}

fn append_checks(value: &mut Value, profile: &Path, previous: Option<&Result<Value, String>>) {
    let disk_check = match read_json(&profile.join("settings.json")) {
        Ok(saved) => persistence_check("settings-file-persistence", &value["final"]["preferences"], &saved, None),
        Err(error) => json!({"name":"settings-file-persistence","status":"failed","error":error}),
    };
    let restart_check = previous.map(|previous| match previous {
        Ok(previous) if previous["status"] == "passed" => persistence_check(
            "settings-survive-native-restart", &previous["final"]["preferences"], &value["initial"]["preferences"],
            Some(&["text_font", "settings_font_size", "pen_width", "marker_width", "eraser_width", "layout", "toolbar_items", "theme", "global_shortcut_enabled", "capture_layer_only"]),
        ),
        Ok(_) => json!({"name":"settings-survive-native-restart","status":"failed","error":"Previous native report did not pass"}),
        Err(error) => json!({"name":"settings-survive-native-restart","status":"failed","error":error}),
    });
    if let Some(checks) = value["checks"].as_array_mut() {
        checks.push(disk_check);
        checks.extend(restart_check);
        if checks.iter().any(|check| check["status"] != "passed") { value["status"] = "failed".into(); }
    } else {
        value["status"] = "failed".into();
        value["errors"] = json!(["Native WebView report contained no checks"]);
    }
}

// Serialize finalization so a timed-out WebView cannot overwrite the watchdog's
// failure with a late success report. Native window inspection happens before
// this lock so the watchdog also covers a stalled window getter.
fn finish(app: &tauri::AppHandle, report: &Path, done: &Mutex<bool>, value: &Value) {
    let Ok(mut finished) = done.lock() else { return; };
    if *finished { return; }
    *finished = true;
    let written = (|| -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = report.parent() { std::fs::create_dir_all(parent)?; }
        std::fs::write(report, serde_json::to_vec_pretty(value)?)?;
        Ok(())
    })();
    if let Err(error) = &written { eprintln!("Validation report write failed: {error}"); }
    let passed = value["status"] == "passed" && written.is_ok();
    if !passed || std::env::var_os("POINTORY_VALIDATION_KEEP_OPEN").is_none() { app.exit(if passed { 0 } else { 1 }); }
}

pub fn start(app: &tauri::AppHandle) {
    let Some((report, profile)) = paths() else { return; };
    if let Err(error) = app.add_capability(r#"{"identifier":"native-validation","windows":["toolbar","settings","overlay","caption-settings","sharing"],"permissions":["core:event:allow-emit"]}"#) {
        eprintln!("Validation capability failed: {error}"); return;
    }
    // Supply the first successful report only for the second app process, using
    // the same isolated profile. Compare before the script configures anything.
    let previous = std::env::var_os("POINTORY_VALIDATION_PREVIOUS_REPORT").map(|path| {
        let path = PathBuf::from(path);
        if !path.is_absolute() { return Err("Previous report path must be absolute".into()); }
        read_json(&path)
    });
    eprintln!("Pointory native validation starting: {}", report.display());
    let done = Arc::new(Mutex::new(false));
    let progress = Arc::new(Mutex::new(String::from("waiting-for-toolbar")));
    let last_progress = progress.clone();
    app.listen("pointory-validation-progress", move |event| {
        eprintln!("Validation: {}", event.payload());
        if let Ok(mut progress) = last_progress.lock() { *progress = event.payload().to_string(); }
    });
    let handle = app.clone();
    app.listen("pointory-validation-eval", move |event| {
        let Ok(value) = serde_json::from_str::<Value>(event.payload()) else { return; };
        let (Some(id), Some(label), Some(script)) = (value["id"].as_str(), value["label"].as_str(), value["script"].as_str()) else { return; };
        if !["toolbar", "settings", "overlay", "caption-settings", "sharing"].contains(&label) { return; }
        let response_error = |error: String| { let _ = handle.emit_to("toolbar", "pointory-validation-response", json!({"id":id,"error":error})); };
        let Some(window) = handle.get_webview_window(label) else { response_error(format!("Missing WebView: {label}")); return; };
        let id = serde_json::to_string(id).unwrap();
        let label = serde_json::to_string(label).unwrap();
        let code = format!(r#"(async()=>{{
            if(!window.__POINTORY_VALIDATION_ERRORS){{
                window.__POINTORY_VALIDATION_ERRORS=true;
                const report=e=>window.__TAURI__.event.emit('pointory-validation-ui-error',{{window:{label},error:String(e.reason||e.error||e.message||'Uncaught WebView error')}}).catch(()=>{{}});
                window.addEventListener('error',report);window.addEventListener('unhandledrejection',report);
            }}
            try{{const data=await (async()=>{{{script}}})();await window.__TAURI__.event.emit('pointory-validation-response',{{id:{id},data}});}}
            catch(e){{await window.__TAURI__.event.emit('pointory-validation-response',{{id:{id},error:String(e)}});}}
        }})()"#);
        if let Err(error) = window.eval(&code) { response_error(error.to_string()); }
    });
    let handle = app.clone();
    let completed = done.clone();
    let report_path = report.clone();
    app.listen("pointory-validation-finished", move |event| {
        eprintln!("Validation received final report");
        let mut value = serde_json::from_str::<Value>(event.payload()).ok().filter(Value::is_object)
            .unwrap_or_else(|| json!({"status":"failed","errors":["Invalid native WebView report"]}));
        let windows: Vec<_> = ["toolbar", "settings", "overlay"].iter().filter_map(|label| {
            let w = handle.get_webview_window(label)?;
            Some(json!({"label":label,"position":w.outer_position().ok(),"size":w.outer_size().ok(),"visible":w.is_visible().ok()}))
        }).collect();
        value["native_windows"] = json!(windows);
        append_checks(&mut value, &profile, previous.as_ref());
        finish(&handle, &report_path, &completed, &value);
    });
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(180));
        let progress = progress.lock().map(|value| value.clone()).unwrap_or_default();
        let value = json!({"status":"failed","checks":[],"errors":["Native validation exceeded 180 seconds"],"last_progress":progress});
        finish(&handle, &report, &done, &value);
    });
}

pub fn page_loaded(window: &tauri::WebviewWindow) {
    if paths().is_none() { return; }
    eprintln!("Validation injecting into loaded page: {:?}", window.url());
    let screen = std::env::var("POINTORY_VALIDATION_SCREEN").as_deref() == Ok("1");
    let script = format!("window.__POINTORY_VALIDATION_SCREEN={screen};\n{}", include_str!("../../tests/native-smoke.js"));
    if let Err(error) = window.eval(&script) { eprintln!("Validation injection failed: {error}"); }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persistence_checks_reject_missing_or_changed_preferences() {
        let expected = json!({"text_font":"Arial","settings_font_size":20,"pen_width":16});
        assert_eq!(persistence_check("saved", &expected, &expected, None)["status"], "passed");
        assert_eq!(persistence_check("saved", &Value::Null, &Value::Null, None)["status"], "failed");
        let changed = json!({"text_font":"Arial","settings_font_size":14,"pen_width":16});
        assert_eq!(persistence_check("saved", &expected, &changed, None)["status"], "failed");
        assert_eq!(persistence_check("restart", &expected, &changed, Some(&["text_font", "settings_font_size"]))["status"], "failed");
        assert_eq!(persistence_check("restart", &expected, &expected, Some(&["missing"]))["status"], "failed");
    }
    #[test]
    fn restart_comparison_ignores_unrelated_monitor_identity() {
        let expected = json!({"text_font":"Arial","settings_font_size":20,"monitor":"old"});
        let actual = json!({"text_font":"Arial","settings_font_size":20,"monitor":"new"});
        assert_eq!(persistence_check("restart", &expected, &actual, Some(&["text_font", "settings_font_size"]))["status"], "passed");
        assert_eq!(persistence_check("disk", &expected, &actual, None)["status"], "failed");
    }
    #[test]
    fn persistence_accepts_integer_and_float_representations_recursively() {
        let expected = json!({"pen_width":31,"marker_width":4,"nested":{"values":[1,2.5,null]},"toolbar_items":["pen","capture"]});
        let actual = json!({"pen_width":31.0,"marker_width":4.0,"nested":{"values":[1.0,2.5,null]},"toolbar_items":["pen","capture"]});
        assert_ne!(expected, actual);
        assert_eq!(persistence_check("disk", &expected, &actual, None)["status"], "passed");
        assert_eq!(persistence_check("restart", &expected, &actual, Some(&["pen_width", "marker_width", "nested", "toolbar_items"]))["status"], "passed");
    }
    #[test]
    fn numeric_normalization_still_rejects_changed_widths_and_toolbar_order() {
        let expected = json!({"pen_width":7.25,"toolbar_items":["pen","capture"]});
        let changed_width = json!({"pen_width":7.5,"toolbar_items":["pen","capture"]});
        let changed_order = json!({"pen_width":7.25,"toolbar_items":["capture","pen"]});
        for actual in [&changed_width, &changed_order] {
            assert_eq!(persistence_check("disk", &expected, actual, None)["status"], "failed");
            assert_eq!(persistence_check("restart", &expected, actual, Some(&["pen_width", "toolbar_items"]))["status"], "failed");
        }
    }
    #[test]
    fn numeric_normalization_preserves_object_keys_types_and_null_checks() {
        let expected = json!({"nested":{"width":4},"monitor":null});
        for actual in [
            json!({"nested":{},"monitor":null}),
            json!({"nested":{"width":4.0,"extra":null},"monitor":null}),
            json!({"nested":{"width":"4"},"monitor":null}),
            json!({"nested":{"width":4.0}}),
        ] {
            assert_eq!(persistence_check("disk", &expected, &actual, None)["status"], "failed");
        }
        assert_eq!(persistence_check("restart", &expected, &expected, Some(&["monitor"]))["status"], "failed");
    }
}
