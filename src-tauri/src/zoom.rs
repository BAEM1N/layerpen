use super::*;
#[tauri::command]
pub async fn zoom_image(app: tauri::AppHandle, id: u64) -> Result<String> {
    let state = app.state::<Shared>();
    let s = state.lock().map_err(|e| e.to_string())?;
    s.zoom
        .as_ref()
        .filter(|z| z.id == id)
        .map(|z| z.image.clone())
        .ok_or("확대 화면이 변경되었습니다.".into())
}
#[tauri::command]
pub async fn start_zoom(
    app: tauri::AppHandle,
    monitor: String,
    scale: f64,
    x: f64,
    y: f64,
) -> Result<()> {
    if !scale.is_finite()
        || !(1.1..=6.).contains(&scale)
        || !x.is_finite()
        || !y.is_finite()
        || !(0.0..=1.0).contains(&x)
        || !(0.0..=1.0).contains(&y)
    {
        return Err("확대 영역을 확인하세요.".into());
    }
    let (display, board) = {
        let state = app.state::<Shared>();
        let mut s = state.lock().map_err(|e| e.to_string())?;
        if s.capturing || !s.drawing || s.prefs.monitor.as_deref() != Some(&monitor) {
            return Err("확대할 모니터 상태가 변경되었습니다.".into());
        }
        let display = s.selected().ok_or("모니터 연결이 해제되었습니다.")?.clone();
        let board = s.active_board().to_string();
        s.capturing = true;
        drawing(&app, &mut s, false)?;
        publish(&app, &s)?;
        (display, board)
    };
    let handle = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let ratio = (4096. / f64::from(display.width.max(display.height))).min(1.);
        let w = (f64::from(display.width) * ratio).round() as u16;
        let h = (f64::from(display.height) * ratio).round() as u16;
        if board == "screen" {
            animation::capture_background(&handle, &display, w, h)
        } else {
            animation::solid_background(w, h, &board)
        }
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|r| r);
    let state = app.state::<Shared>();
    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.capturing = false;
    match result {
        Ok(image) => {
            if s.prefs.monitor.as_deref() != Some(&monitor) || s.selected().is_none() {
                publish(&app, &s)?;
                return Err("모니터가 변경되어 확대를 취소했습니다.".into());
            }
            s.zoom = Some(ZoomView {
                id: now_ms(),
                scale,
                x: x.clamp(0., 1. - 1. / scale),
                y: y.clamp(0., 1. - 1. / scale),
                image,
            });
            s.tool = s.zoom_return_tool.clone();
            s.width = s.zoom_return_width;
            drawing(&app, &mut s, true)?;
            publish(&app, &s)
        }
        Err(e) => {
            s.zoom = None;
            publish(&app, &s)?;
            Err(e)
        }
    }
}
