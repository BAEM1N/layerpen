use super::*;
use base64::Engine;
use std::io::Write;
use tauri_plugin_dialog::DialogExt;

/// Request normal macOS consent only when the user asks to capture the screen.
/// CoreGraphics can otherwise return a valid image containing only the desktop
/// and our own windows, which must not be reported as a successful screen share.
pub(crate) fn ensure_capture_permission() -> Result<()> {
    #[cfg(target_os = "macos")]
    if !objc2_core_graphics::CGPreflightScreenCaptureAccess() {
        let _ = objc2_core_graphics::CGRequestScreenCaptureAccess();
        return Err("Screen Recording permission is required. Allow Pointory in System Settings → Privacy & Security → Screen & System Audio Recording, then retry. Restart Pointory if needed.\n화면 기록 권한이 필요합니다. 시스템 설정에서 Pointory를 허용한 뒤 다시 시도하세요. 필요하면 앱을 다시 실행하세요.".into());
    }
    Ok(())
}

// On macOS Tauri scales the global display origin by that display's backing
// scale, while xcap returns CGDisplayBounds in logical points. Windows and
// Linux already use matching coordinates and must keep their physical origin.
fn capture_origin(x: i32, y: i32, coordinate_scale: f64) -> (i32, i32) {
    (
        (f64::from(x) / coordinate_scale).round() as i32,
        (f64::from(y) / coordinate_scale).round() as i32,
    )
}

pub(crate) fn monitor_for_display(display: &Display) -> Result<xcap::Monitor> {
    let scale = if cfg!(target_os = "macos") { display.scale } else { 1. };
    let (x, y) = capture_origin(display.x, display.y, scale);
    xcap::Monitor::all()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|m| m.x().ok() == Some(x) && m.y().ok() == Some(y))
        .ok_or_else(|| "선택한 모니터를 캡처할 수 없습니다.".into())
}

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
                ensure_capture_permission()?;
                toolbar.hide().map_err(|e| e.to_string())?;
                settings.hide().map_err(|e| e.to_string())?;
                // Let the window manager composite the desktop without our controls.
                std::thread::sleep(Duration::from_millis(300));
                let target = monitor_for_display(&display)?;
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

#[cfg(test)]
mod tests {
    use super::capture_origin;

    #[test]
    fn retina_capture_uses_logical_secondary_display_origin() {
        // A 2x display placed 1,440 points right of the main display is reported
        // by Tauri at 2,880 physical pixels; xcap identifies it at 1,440 points.
        assert_eq!(capture_origin(2880, 200, 2.), (1440, 100));
        assert_ne!(capture_origin(2880, 200, 2.), (2880, 200));
        assert_eq!(capture_origin(-2880, -1800, 2.), (-1440, -900));
        assert_eq!(capture_origin(0, 0, 2.), (0, 0));
    }

    #[test]
    fn unscaled_capture_preserves_physical_origins() {
        for origin in [(0, 0), (1920, 120), (-1920, -1080)] {
            assert_eq!(capture_origin(origin.0, origin.1, 1.), origin);
        }
    }
}
