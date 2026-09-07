use super::*;
use base64::Engine;
use std::{fs::File, path::PathBuf};
#[derive(Default)]
pub struct ExportState(pub Mutex<Option<Job>>);
pub struct Job {
    encoder: gif::Encoder<File>,
    temp: PathBuf,
    path: PathBuf,
    width: u16,
    height: u16,
    frames: u32,
}
#[tauri::command]
pub async fn begin_gif(
    app: tauri::AppHandle,
    width: u16,
    height: u16,
) -> Result<serde_json::Value> {
    if width == 0 || height == 0 || u32::from(width) * u32::from(height) > 1_000_000 {
        return Err("GIF 크기가 너무 큽니다.".into());
    }
    let (data, background_display, zoom_background) = {
        let state = app.state::<Shared>();
        let mut s = state.lock().map_err(|e| e.to_string())?;
        if s.capturing {
            return Err("이미 내보내는 중입니다.".into());
        }
        let board = s.active_board().to_string();
        let zoom_background = if s.prefs.gif_background == "screen" && s.drawing {
            s.zoom.as_ref().map(|z| z.image.clone())
        } else {
            None
        };
        let display = s.selected().ok_or("모니터 연결이 해제되었습니다.")?.clone();
        let id = s.prefs.monitor.as_ref().ok_or("모니터를 선택하세요.")?;
        let history = s.documents.get(id).ok_or("기록된 필기가 없습니다.")?;
        if history.recording.is_empty() {
            return Err("기록된 필기가 없습니다.".into());
        }
        let data = serde_json::to_value(&history.recording).map_err(|e| e.to_string())?;
        let folder = PathBuf::from(&s.prefs.capture_dir);
        if !folder.is_absolute() {
            return Err("설정에서 저장 폴더를 선택하세요.".into());
        }
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let path = folder.join(format!(
            "{}.gif",
            chrono::Local::now().format("%Y-%m-%d_%H-%M-%S-%6f")
        ));
        let temp = path.with_extension("gif.part");
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        s.capturing = true;
        let result = (|| -> Result<gif::Encoder<File>> {
            let mut encoder =
                gif::Encoder::new(file, width, height, &[]).map_err(|e| e.to_string())?;
            if s.prefs.gif_repeat {
                encoder
                    .set_repeat(gif::Repeat::Infinite)
                    .map_err(|e| e.to_string())?;
            }
            drawing(&app, &mut s, false)?;
            Ok(encoder)
        })();
        let encoder = match result {
            Ok(e) => e,
            Err(e) => {
                s.capturing = false;
                let _ = std::fs::remove_file(&temp);
                return Err(e);
            }
        };
        *app.state::<ExportState>()
            .0
            .lock()
            .map_err(|e| e.to_string())? = Some(Job {
            encoder,
            temp,
            path,
            width,
            height,
            frames: 0,
        });
        s.capturing = true;
        publish(&app, &s)?;
        (
            data,
            if s.prefs.gif_background == "screen" {
                Some((display, board))
            } else {
                None
            },
            zoom_background,
        )
    };
    let background = if zoom_background.is_some() {
        zoom_background
    } else if let Some((display, board)) = background_display {
        let handle = app.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            if board == "screen" {
                capture_background(&handle, &display, width, height)
            } else {
                solid_background(width, height, &board)
            }
        })
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| r);
        match result {
            Ok(png) => Some(png),
            Err(e) => {
                let _ = abort_gif(app.clone()).await;
                return Err(e);
            }
        }
    } else {
        None
    };
    Ok(serde_json::json!({"events":data,"background":background}))
}
#[tauri::command]
pub async fn gif_frame(app: tauri::AppHandle, png: String, delay: u16) -> Result<()> {
    if png.len() > 8_000_000 || delay == 0 {
        return Err("Invalid GIF frame".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(
                png.strip_prefix("data:image/png;base64,")
                    .ok_or("Invalid PNG")?,
            )
            .map_err(|e| e.to_string())?;
        let image =
            xcap::image::load_from_memory_with_format(&bytes, xcap::image::ImageFormat::Png)
                .map_err(|e| e.to_string())?
                .to_rgba8();
        let export = app.state::<ExportState>();
        let mut job = export.0.lock().map_err(|e| e.to_string())?;
        let j = job.as_mut().ok_or("GIF 내보내기가 종료되었습니다.")?;
        if image.width() != u32::from(j.width)
            || image.height() != u32::from(j.height)
            || j.frames >= 3000
        {
            return Err("GIF 프레임 제한을 초과했습니다.".into());
        }
        let mut rgba = image.into_raw();
        let mut frame = gif::Frame::from_rgba_speed(j.width, j.height, &mut rgba, 10);
        frame.delay = delay;
        frame.dispose = gif::DisposalMethod::Background;
        j.encoder.write_frame(&frame).map_err(|e| e.to_string())?;
        j.frames += 1;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
fn unlock(app: &tauri::AppHandle) -> Result<()> {
    let state = app.state::<Shared>();
    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.capturing = false;
    s.zoom = None;
    publish(app, &s)
}
#[tauri::command]
pub async fn finish_gif(app: tauri::AppHandle) -> Result<String> {
    let job = app
        .state::<ExportState>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .take()
        .ok_or("GIF 내보내기가 없습니다.")?;
    let Job {
        encoder,
        temp,
        path,
        frames,
        ..
    } = job;
    let result = (|| -> Result<String> {
        let file = encoder.into_inner().map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        if frames == 0 {
            return Err("GIF 프레임이 없습니다.".into());
        }
        std::fs::rename(&temp, &path).map_err(|e| e.to_string())?;
        Ok(path.to_string_lossy().into_owned())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    unlock(&app)?;
    if let Ok(path) = &result {
        let _ = app.emit("capture-saved", path);
    }
    result
}
#[tauri::command]
pub async fn abort_gif(app: tauri::AppHandle) -> Result<()> {
    let job = app
        .state::<ExportState>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .take();
    if let Some(Job { encoder, temp, .. }) = job {
        drop(encoder);
        let _ = std::fs::remove_file(temp);
        unlock(&app)?;
    }
    Ok(())
}

// Capture one clean desktop image for the entire replay. Restore every window even on errors.
pub(crate) fn capture_background(
    app: &tauri::AppHandle,
    display: &Display,
    width: u16,
    height: u16,
) -> Result<String> {
    let mut windows = Vec::new();
    for label in ["overlay", "toolbar", "settings"] {
        let w = app
            .get_webview_window(label)
            .ok_or("창을 찾을 수 없습니다.")?;
        let visible = w.is_visible().map_err(|e| e.to_string())?;
        windows.push((w, visible));
    }
    let result = (|| -> Result<String> {
        for (w, _) in windows.iter().rev() {
            w.hide().map_err(|e| e.to_string())?;
        }
        std::thread::sleep(Duration::from_millis(300));
        let target = xcap::Monitor::all()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|m| m.x().ok() == Some(display.x) && m.y().ok() == Some(display.y))
            .ok_or("선택한 화면을 캡처할 수 없습니다.")?;
        let image = target
            .capture_image()
            .map_err(|e| format!("화면 캡처 실패: {e}"))?;
        let resized = xcap::image::imageops::resize(
            &image,
            u32::from(width),
            u32::from(height),
            xcap::image::imageops::FilterType::Triangle,
        );
        let mut bytes = std::io::Cursor::new(Vec::new());
        resized
            .write_to(&mut bytes, xcap::image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        Ok(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes.into_inner())
        ))
    })();
    let mut restore_error = None;
    for (w, visible) in windows {
        if visible {
            if let Err(e) = w.show() {
                restore_error = Some(e.to_string());
            }
        }
    }
    if let Some(e) = restore_error {
        return Err(format!("캡처 후 창 복원 실패: {e}"));
    }
    result
}

pub(crate) fn solid_background(width: u16, height: u16, board: &str) -> Result<String> {
    let color = if board == "white" {
        [255, 255, 255, 255]
    } else {
        [23, 23, 23, 255]
    };
    let image =
        xcap::image::RgbaImage::from_pixel(width.into(), height.into(), xcap::image::Rgba(color));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, xcap::image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes.into_inner())
    ))
}
