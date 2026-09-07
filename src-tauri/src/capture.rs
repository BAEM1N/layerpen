use super::*;
use base64::Engine;
use std::io::Write;
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn choose_capture_folder(app: tauri::AppHandle, title: Option<String>) -> Result<Option<String>> {
    app.dialog()
        .file()
        .set_title(title.as_deref().unwrap_or("Screenshot folder"))
        .blocking_pick_folder()
        .map(|p| {
            p.into_path()
                .map(|p| p.to_string_lossy().into_owned())
                .map_err(|e| e.to_string())
        })
        .transpose()
}
#[tauri::command]
pub async fn request_capture(app: tauri::AppHandle) -> Result<()> {
    let state = app.state::<Shared>();
    let mut s = state.lock().map_err(|e| e.to_string())?;
    if s.capturing {
        return Err("캡처를 저장하는 중입니다.".into());
    }
    if s.selected().is_none() {
        return Err("캡처할 모니터를 선택하세요.".into());
    }
    s.capture_board = s.active_board().into();
    s.capturing = true;
    if let Err(e) = drawing(&app, &mut s, false) {
        s.capturing = false;
        return Err(e);
    }
    publish(&app, &s)?;
    if let Err(e) = app.emit_to("overlay", "capture-request", ()) {
        s.capturing = false;
        let _ = publish(&app, &s);
        return Err(e.to_string());
    }
    Ok(())
}
#[tauri::command]
pub async fn cancel_capture(app: tauri::AppHandle, message: String) -> Result<()> {
    let state = app.state::<Shared>();
    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.capturing = false;
    s.zoom = None;
    publish(&app, &s)?;
    app.emit("capture-error", message)
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn save_capture(app: tauri::AppHandle, monitor: String, png: String) -> Result<String> {
    let (display, prefs, board) = {
        let state = app.state::<Shared>();
        let s = state.lock().map_err(|e| e.to_string())?;
        if !s.capturing || s.prefs.monitor.as_deref() != Some(&monitor) {
            return Err("캡처 도중 모니터가 변경되었습니다.".into());
        }
        (
            s.selected().ok_or("모니터 연결이 해제되었습니다.")?.clone(),
            s.prefs.clone(),
            s.capture_board.clone(),
        )
    };
    let handle = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || -> Result<String> {
        let toolbar = handle
            .get_webview_window("toolbar")
            .ok_or("Toolbar unavailable")?;
        let settings = handle
            .get_webview_window("settings")
            .ok_or("Settings unavailable")?;
        let settings_visible = settings.is_visible().map_err(|e| e.to_string())?;
        let result = (|| -> Result<String> {
            let image = if prefs.capture_layer_only || board != "screen" {
                if png.len() > 100_000_000 {
                    return Err("캡처 이미지가 너무 큽니다.".into());
                }
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(
                        png.strip_prefix("data:image/png;base64,")
                            .ok_or("Invalid PNG")?,
                    )
                    .map_err(|e| e.to_string())?;
                if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
                    return Err("Invalid PNG".into());
                }
                if !prefs.capture_layer_only && board != "screen" {
                    let layer = xcap::image::load_from_memory(&bytes)
                        .map_err(|e| e.to_string())?
                        .to_rgba8();
                    let color = if board == "white" {
                        [255, 255, 255, 255]
                    } else {
                        [23, 23, 23, 255]
                    };
                    let mut image = xcap::image::RgbaImage::from_pixel(
                        layer.width(),
                        layer.height(),
                        xcap::image::Rgba(color),
                    );
                    xcap::image::imageops::overlay(&mut image, &layer, 0, 0);
                    let mut out = std::io::Cursor::new(Vec::new());
                    image
                        .write_to(&mut out, xcap::image::ImageFormat::Png)
                        .map_err(|e| e.to_string())?;
                    out.into_inner()
                } else {
                    bytes
                }
            } else {
                toolbar.hide().map_err(|e| e.to_string())?;
                settings.hide().map_err(|e| e.to_string())?;
                // Let the window manager composite the desktop without our controls.
                std::thread::sleep(Duration::from_millis(300));
                let target = xcap::Monitor::all()
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .find(|m| m.x().ok() == Some(display.x) && m.y().ok() == Some(display.y))
                    .ok_or("선택한 모니터를 캡처할 수 없습니다.")?;
                let image = target
                    .capture_image()
                    .map_err(|e| format!("화면 캡처 실패. 화면 기록 권한을 확인하세요: {e}"))?;
                let mut bytes = std::io::Cursor::new(Vec::new());
                image
                    .write_to(&mut bytes, xcap::image::ImageFormat::Png)
                    .map_err(|e| e.to_string())?;
                bytes.into_inner()
            };
            let folder = std::path::PathBuf::from(&prefs.capture_dir);
            if !folder.is_absolute() {
                return Err("설정에서 저장 폴더를 선택하세요.".into());
            }
            std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
            let filename = format!(
                "{}.png",
                chrono::Local::now().format("%Y-%m-%d_%H-%M-%S-%6f")
            );
            let path = folder.join(filename);
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|e| e.to_string())?;
            file.write_all(&image).map_err(|e| e.to_string())?;
            Ok(path.to_string_lossy().into_owned())
        })();
        let _ = toolbar.show();
        if settings_visible {
            let _ = settings.show();
        }
        result
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|r| r);
    {
        let state = app.state::<Shared>();
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.capturing = false;
        s.zoom = None;
        publish(&app, &s)?;
    }
    match &result {
        Ok(path) => {
            let _ = app.emit("capture-saved", path);
        }
        Err(e) => {
            let _ = app.emit("capture-error", e);
        }
    }
    result
}
