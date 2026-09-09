#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod animation;
mod sharing;
mod spotlight;
mod docking;
mod fonts;
mod captions;
mod capture;
mod geometry;
mod model;
mod profile;
mod zoom;
use model::*;
use std::{sync::Mutex, time::Duration};
use tauri::{
    Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
type Shared = Mutex<Session>;
type Result<T> = std::result::Result<T, String>;

fn profile_override() -> Option<std::path::PathBuf> {
    profile::override_directory(|key| std::env::var_os(key))
}
fn settings_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf> {
    if let Some(root) = profile_override() { return Ok(root.join("settings.json")); }
    Ok(app
        .path()
        .config_dir()
        .map_err(|e| e.to_string())?
        .join("Pointory").join("settings.json"))
}
fn webview_data(app: &tauri::AppHandle) -> Result<std::path::PathBuf> {
    if let Some(root) = profile_override() { return Ok(root.join("webview")); }
    app.path().app_local_data_dir().map_err(|e| e.to_string())
}
fn save(app: &tauri::AppHandle, prefs: &Preferences) -> Result<()> {
    let path = settings_path(app)?;
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(
        path,
        serde_json::to_vec_pretty(prefs).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
fn displays(app: &tauri::AppHandle) -> Result<Vec<Display>> {
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    Ok(monitors
        .iter()
        .map(|m| {
            let name = m.name().cloned().unwrap_or_else(|| "Display".into());
            let count = monitors.iter().filter(|n| n.name() == m.name()).count();
            // Qt/Tauri monitor names are the best portable identifier available. Duplicate names
            // need a position suffix; rearranging identical displays requires reselection.
            let id = if count == 1 {
                name.clone()
            } else {
                format!("{}@{},{}", name, m.position().x, m.position().y)
            };
            Display {
                id,
                name,
                x: m.position().x,
                y: m.position().y,
                width: m.size().width,
                height: m.size().height,
                scale: m.scale_factor(),
            }
        })
        .collect())
}
fn publish(app: &tauri::AppHandle, s: &Session) -> Result<()> {
    app.emit("session", s.snapshot()).map_err(|e| e.to_string())
}
fn dock_settings(app: &tauri::AppHandle) -> Result<()> {
    let Some(toolbar)=app.get_webview_window("toolbar") else {return Ok(());};
    let Some(panel)=app.get_webview_window("settings") else {return Ok(());};
    let Some(monitor)=toolbar.current_monitor().map_err(|e|e.to_string())? else {return Ok(());};
    let scale=monitor.scale_factor();let area=monitor.work_area();
    let width=(520.*scale).round().min(area.size.width as f64) as u32;
    let height=(700.*scale).round().min(area.size.height as f64) as u32;
    panel.set_size(PhysicalSize::new(width,height)).map_err(|e|e.to_string())?;
    let pos=toolbar.outer_position().map_err(|e|e.to_string())?;
    let size=toolbar.outer_size().map_err(|e|e.to_string())?;
    let (x,y)=docking::position(docking::Rect{x:pos.x,y:pos.y,w:size.width as i32,h:size.height as i32},
      docking::Rect{x:area.position.x,y:area.position.y,w:area.size.width as i32,h:area.size.height as i32},width as i32,height as i32,(8.*scale).round() as i32);
    panel.set_position(PhysicalPosition::new(x,y)).map_err(|e|e.to_string())
}
fn show_settings(app: &tauri::AppHandle) -> Result<()> {
    dock_settings(app)?;
    let w = app
        .get_webview_window("settings")
        .ok_or("Settings window unavailable")?;
    w.show().map_err(|e| e.to_string())?;
    w.set_focus().map_err(|e| e.to_string())
}
fn place_toolbar(app: &tauri::AppHandle, s: &Session) -> Result<()> {
    let Some(m) = s.selected() else {
        return Ok(());
    };
    let toolbar = app
        .get_webview_window("toolbar")
        .ok_or("Toolbar unavailable")?;
    let vertical = s.prefs.layout == "vertical";
    let (natural_width,natural_height)=docking::toolbar_size(vertical);
    let width=natural_width.min(m.width as f64/m.scale-16.).max(48.);
    let height=natural_height.min(m.height as f64/m.scale-48.).max(48.);
    toolbar
        .set_size(LogicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    let x = if vertical {
        m.x + (24. * m.scale) as i32
    } else {
        m.x + ((m.width as f64 - width * m.scale).max(0.) / 2.) as i32
    };
    let y = if vertical {
        m.y + ((m.height as f64 - height * m.scale).max(0.) / 2.) as i32
    } else {
        m.y + (24. * m.scale) as i32
    };
    toolbar
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    toolbar.show().map_err(|e| e.to_string())
}
#[tauri::command]
async fn toolbar_panel(app: tauri::AppHandle, open: bool) -> Result<()> {
    let vertical = {
        let state = app.state::<Shared>();
        let s = state.lock().map_err(|e| e.to_string())?;
        s.prefs.layout == "vertical"
    };
    let toolbar = app.get_webview_window("toolbar").ok_or("Toolbar unavailable")?;
    let monitor = toolbar.current_monitor().map_err(|e| e.to_string())?
        .ok_or("Toolbar monitor unavailable")?;
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let current = toolbar.outer_position().map_err(|e| e.to_string())?;
    let (natural_width, natural_height) = docking::toolbar_panel_size(vertical, open);
    let width = natural_width.min(area.size.width as f64 / scale);
    let height = natural_height.min(area.size.height as f64 / scale);
    let max_x = area.position.x + area.size.width as i32 - (width * scale).round() as i32;
    let max_y = area.position.y + area.size.height as i32 - (height * scale).round() as i32;
    let x = current.x.clamp(area.position.x, max_x.max(area.position.x));
    let y = current.y.clamp(area.position.y, max_y.max(area.position.y));
    toolbar.set_size(LogicalSize::new(width, height)).map_err(|e| e.to_string())?;
    toolbar.set_position(PhysicalPosition::new(x, y)).map_err(|e| e.to_string())?;
    if app.get_webview_window("settings").is_some_and(|w| w.is_visible().unwrap_or(false)) {
        dock_settings(&app)?;
    }
    Ok(())
}
fn place(app: &tauri::AppHandle, s: &mut Session) -> Result<()> {
    s.drawing = false;
    s.zoom = None;
    let overlay = app
        .get_webview_window("overlay")
        .ok_or("Overlay unavailable")?;
    overlay
        .set_ignore_cursor_events(true)
        .map_err(|e| e.to_string())?;
    overlay.hide().map_err(|e| e.to_string())?;
    if let Some(m) = s.selected() {
        overlay
            .set_position(PhysicalPosition::new(m.x, m.y))
            .map_err(|e| e.to_string())?;
        overlay
            .set_size(PhysicalSize::new(m.width, m.height))
            .map_err(|e| e.to_string())?;
        overlay.show().map_err(|e| e.to_string())?;
        let toolbar = app
            .get_webview_window("toolbar")
            .ok_or("Toolbar unavailable")?;
        place_toolbar(app, s)?;
        toolbar.set_always_on_top(true).map_err(|e| e.to_string())?;
        toolbar.show().map_err(|e| e.to_string())?;
    } else {
        show_settings(app)?;
    }
    Ok(())
}
fn drawing(app: &tauri::AppHandle, s: &mut Session, enabled: bool) -> Result<()> {
    if enabled && s.capturing {
        return Err("캡처를 저장하는 중입니다.".into());
    }
    if enabled && s.selected().is_none() {
        return Err("선택한 모니터가 연결되어 있지 않습니다. 설정에서 다시 선택하세요.".into());
    }
    let w = app
        .get_webview_window("overlay")
        .ok_or("Overlay unavailable")?;
    w.set_ignore_cursor_events(!enabled)
        .map_err(|e| e.to_string())?;
    s.drawing = enabled;
    if !enabled && !s.capturing {
        s.zoom = None;
    }
    if enabled {
        s.visible = true;
        w.set_focus().map_err(|e| e.to_string())?;
        if let Some(t) = app.get_webview_window("toolbar") {
            t.set_always_on_top(true).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
#[tauri::command]
async fn snapshot(app: tauri::AppHandle) -> Result<serde_json::Value> {
    Ok(app
        .state::<Shared>()
        .lock()
        .map_err(|e| e.to_string())?
        .snapshot())
}
#[tauri::command]
async fn action(app: tauri::AppHandle, name: String) -> Result<()> {
    if name == "quit" {
        captions::stop(&app);
        app.exit(0);
        return Ok(());
    }
    let state = app.state::<Shared>();
    let mut s = state.lock().map_err(|e| e.to_string())?;
    if s.capturing && matches!(name.as_str(), "undo" | "redo" | "clear") {
        return Err("내보내기 완료 후 편집하세요.".into());
    }
    match name.as_str() {
        "settings" => {
            drawing(&app, &mut s, false)?;
            let panel=app.get_webview_window("settings").ok_or("Settings window unavailable")?;
            if panel.is_visible().unwrap_or(false){panel.hide().map_err(|e|e.to_string())?;}else{show_settings(&app)?;}
        }
        "toggle" => {
            let enabled = !s.drawing;
            drawing(&app, &mut s, enabled)?;
        }
        "zoom_reset" => {
            if s.capturing {
                return Err("내보내기 후 확대를 종료하세요.".into());
            }
            s.zoom = None;
        }
        "board" | "board_white" | "board_black" | "board_screen" => {
            if s.capturing {
                return Err("내보내기 후 보드를 바꾸세요.".into());
            }
            s.zoom = None;
            let next = match name.as_str() {
                "board_white" => "white",
                "board_black" => "black",
                "board_screen" => "screen",
                _ => match s.board() {
                    "screen" => "white",
                    "white" => "black",
                    _ => "screen",
                },
            };
            let id = s.prefs.monitor.clone().ok_or("모니터를 선택하세요.")?;
            drawing(&app, &mut s, true)?;
            s.boards.insert(id, next.into());
            if next == "white" && matches!(s.color.as_str(), "#ffffff" | "#f8fafc") {
                s.color = "#171717".into();
            }
            if next == "black" && matches!(s.color.as_str(), "#000000" | "#171717") {
                s.color = "#f8fafc".into();
            }
        }
        "mouse" => drawing(&app, &mut s, false)?,
        "toggle_visibility" => {
            if s.capturing {
                return Err("캡처 저장 후 표시 상태를 바꾸세요.".into());
            }
            drawing(&app, &mut s, false)?;
            s.visible = !s.visible;
        }
        "undo" | "redo" | "clear" => {
            if let Some(id) = s.prefs.monitor.clone() {
                if name == "clear" {
                    s.fading.remove(&id);
                }
                let h = s.documents.entry(id).or_default();
                match name.as_str() {
                    "undo" => h.undo(),
                    "redo" => h.redo(),
                    _ => {
                        h.clear();
                    }
                }
            }
        }
        _ => return Err("Unknown action".into()),
    }
    publish(&app, &s)
}
#[tauri::command]
async fn select_monitor(app: tauri::AppHandle, id: String) -> Result<()> {
    let state = app.state::<Shared>();
    let mut s = state.lock().map_err(|e| e.to_string())?;
    if s.capturing {
        return Err("캡처 저장 후 모니터를 변경하세요.".into());
    }
    s.displays = displays(&app)?;
    if !s.displays.iter().any(|m| m.id == id) {
        return Err("모니터 연결이 변경되었습니다. 다시 선택하세요.".into());
    }
    let previous = s.prefs.clone();
    s.prefs.monitor = Some(id);
    if let Err(e) = place(&app, &mut s).and_then(|_| save(&app, &s.prefs)) {
        s.prefs = previous;
        let _ = place(&app, &mut s);
        let _ = publish(&app, &s);
        return Err(e);
    }
    publish(&app, &s)
}
#[tauri::command]
async fn set_tool(app: tauri::AppHandle, tool: String, color: String, width: f64) -> Result<()> {
    let probe = Stroke {
        tool: if tool == "select" || tool == "zoom" || tool == "text" {
            "pen".into()
        } else {
            tool.clone()
        },
        text: None,
        times: vec![],
        color: color.clone(),
        width,
        opacity: 0.3,
        points: vec![Point { x: 0., y: 0. }],
    };
    if !probe.valid() {
        return Err("Invalid tool".into());
    }
    let state = app.state::<Shared>();
    let mut s = state.lock().map_err(|e| e.to_string())?;
    if s.capturing {
        return Err("내보내기 후 도구를 변경하세요.".into());
    }
    if tool == "zoom" {
        if s.tool != "zoom" {
            s.zoom_return_width = s.width;
            s.zoom_return_tool = if s.tool == "select" {
                "pen".into()
            } else {
                s.tool.clone()
            };
        }
        s.zoom = None;
    }
    s.tool = tool;
    s.color = color;
    s.width = width;
    publish(&app, &s)
}
#[tauri::command]
async fn configure(app: tauri::AppHandle, mut preferences: Preferences) -> Result<()> {
    if !preferences.valid() {
        return Err("설정 값을 확인하세요. 켜진 단축키는 서로 중복될 수 없으며, 전역 단축키는 Ctrl/⌘·Alt 조합 또는 F1–F12여야 합니다.".into());
    }
    let state = app.state::<Shared>();
    let mut s = state.lock().map_err(|e| e.to_string())?;
    if s.capturing {
        return Err("캡처 저장 후 설정을 변경하세요.".into());
    }
    let previous = s.prefs.clone();
    preferences.monitor = previous.monitor.clone();
    let changed_shortcut = preferences.shortcut != previous.shortcut
        || preferences.global_shortcut_enabled != previous.global_shortcut_enabled;
    if changed_shortcut && preferences.global_shortcut_enabled {
        app.global_shortcut()
            .register(preferences.shortcut.as_str())
            .map_err(|e| format!("다른 앱이 이 단축키를 사용 중일 수 있습니다: {e}"))?;
    }
    s.prefs = preferences;
    let result = (|| {
        if s.prefs.layout != previous.layout {
            place_toolbar(&app, &s)?;
        }
        save(&app, &s.prefs)
    })();
    if let Err(e) = result {
        if changed_shortcut && s.prefs.global_shortcut_enabled {
            let _ = app.global_shortcut().unregister(s.prefs.shortcut.as_str());
        }
        s.prefs = previous;
        let _ = place_toolbar(&app, &s);
        return Err(e);
    }
    if changed_shortcut && previous.global_shortcut_enabled {
        let _ = app.global_shortcut().unregister(previous.shortcut.as_str());
    }
    let configured_width = s.prefs.width_for(&s.tool);
    if configured_width != previous.width_for(&s.tool) {
        s.width = configured_width;
    }
    publish(&app, &s)
}
#[tauri::command]
async fn add_stroke(app: tauri::AppHandle, monitor: String, stroke: Stroke) -> Result<()> {
    if !stroke.valid() {
        return Err("Invalid stroke".into());
    }
    let state = app.state::<Shared>();
    let mut s = state.lock().map_err(|e| e.to_string())?;
    // A stroke completed while switching displays must never land on the new display.
    if s.prefs.monitor.as_deref() != Some(&monitor) || !s.drawing {
        return Ok(());
    }
    let display = s.selected().ok_or("모니터 연결이 해제되었습니다.")?;
    let (width, height) = (
        display.width as f64 / display.scale,
        display.height as f64 / display.scale,
    );
    if stroke.tool == "fade" {
        let life = s.prefs.fade_seconds * 1000;
        let now = now_ms();
        let items = s.fading.entry(monitor.clone()).or_default();
        items.retain(|f| now < f.born + f.life);
        if items.len() >= 200 {
            items.remove(0);
        }
        items.push(FadingStroke {
            stroke: stroke.clone(),
            born: now,
            life,
        });
        s.documents
            .entry(monitor)
            .or_default()
            .record_fade(stroke, life);
        return publish(&app, &s);
    }
    let h = s.documents.entry(monitor).or_default();
    if stroke.tool == "eraser" {
        h.erase(&stroke, width, height);
        return publish(&app, &s);
    }
    if h.strokes.len() >= 5000 {
        return Err("필기가 5,000개를 넘었습니다. 전체 지우기 후 계속하세요.".into());
    }
    h.push(stroke);
    publish(&app, &s)
}
#[tauri::command]
async fn move_stroke(
    app: tauri::AppHandle,
    monitor: String,
    index: usize,
    dx: f64,
    dy: f64,
    duration: u64,
) -> Result<()> {
    let state = app.state::<Shared>();
    let mut s = state.lock().map_err(|e| e.to_string())?;
    if s.capturing
        || !s.drawing
        || s.tool != "select"
        || s.prefs.monitor.as_deref() != Some(&monitor)
    {
        return Err("이동 상태가 변경되었습니다.".into());
    }
    if let Some(h) = s.documents.get_mut(&monitor) {
        h.move_stroke(index, dx, dy, duration);
    }
    publish(&app, &s)
}
#[tauri::command]
async fn resize_stroke(
    app: tauri::AppHandle,
    monitor: String,
    index: usize,
    handle: String,
    x: f64,
    y: f64,
    uniform: bool,
    duration: u64,
) -> Result<()> {
    let state = app.state::<Shared>();
    let mut s = state.lock().map_err(|e| e.to_string())?;
    if s.capturing
        || !s.drawing
        || s.tool != "select"
        || s.prefs.monitor.as_deref() != Some(&monitor)
    {
        return Err("크기 조절 상태가 변경되었습니다.".into());
    }
    let m = s.selected().ok_or("모니터 연결이 해제되었습니다.")?;
    let (w, h) = (m.width as f64 / m.scale, m.height as f64 / m.scale);
    if let Some(history) = s.documents.get_mut(&monitor) {
        history.resize_stroke(index, &handle, x, y, uniform, duration, w, h);
    }
    publish(&app, &s)
}
#[tauri::command]
async fn identify(app: tauri::AppHandle) -> Result<()> {
    for (i, m) in displays(&app)?.iter().enumerate() {
        let label = format!("identify-{i}");
        if app.get_webview_window(&label).is_some() {
            continue;
        }
        let w = WebviewWindowBuilder::new(
            &app,
            &label,
            WebviewUrl::App(format!("index.html?view=identify&number={}", i + 1).into()),
        )
        .title("Pointory · 화면 식별")
        .data_directory(webview_data(&app)?)
        .inner_size(180., 130.)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .build()
        .map_err(|e| e.to_string())?;
        w.set_position(PhysicalPosition::new(
            m.x + (m.width as i32 - (180. * m.scale) as i32) / 2,
            m.y + (m.height as i32 - (130. * m.scale) as i32) / 2,
        ))
        .map_err(|e| e.to_string())?;
        w.set_ignore_cursor_events(true)
            .map_err(|e| e.to_string())?;
        let handle = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(3));
            if let Some(w) = handle.get_webview_window(&label) {
                let _ = w.close();
            }
        });
    }
    Ok(())
}
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().with_handler(|app,_,event| {
            if event.state() == ShortcutState::Pressed {
                let state = app.state::<Shared>();
                if let Ok(mut s) = state.lock() { let enabled = !s.drawing; if let Err(e) = drawing(app,&mut s,enabled) { s.warning = Some(e); } let _ = publish(app,&s); };
            }
        }).build())
        .invoke_handler(tauri::generate_handler![toolbar_panel,spotlight::spotlight_toggle,spotlight::spotlight_status,sharing::share_live,sharing::share_open,sharing::share_status,sharing::share_start,sharing::share_stop,sharing::share_add,sharing::share_remove,fonts::system_fonts,fonts::font_assets,fonts::font_data,fonts::import_font,captions::caption_open,captions::caption_start,captions::caption_stop,captions::caption_snapshot,captions::caption_devices,captions::caption_hardware,snapshot,action,select_monitor,set_tool,add_stroke,move_stroke,resize_stroke,zoom::start_zoom,zoom::zoom_image,animation::begin_gif,animation::gif_frame,animation::finish_gif,animation::abort_gif,identify,configure,capture::request_capture,capture::save_capture,capture::cancel_capture,capture::choose_capture_folder])
        .setup(|app| {
            let profile = profile_override();
            let (directory, legacy) = profile::directories(&app.path().config_dir()?, &app.path().app_config_dir()?, profile.clone());
            let prefs = profile::load(&directory, &legacy)?;
            let mut s = Session::new(prefs);
            s.prefs.migrate_capture_dir(&app.path().picture_dir()?, profile.as_deref());
            s.displays = displays(app.handle())?;
            if s.prefs.monitor.is_none() {
                let primary = app.primary_monitor()?;
                s.prefs.monitor = primary.and_then(|p|s.displays.iter().find(|m|m.x==p.position().x && m.y==p.position().y).map(|m|m.id.clone()))
                    .or_else(||s.displays.first().map(|m|m.id.clone()));
            }
            // Webviews can invoke commands before setup returns. Register state first.
            app.manage(Mutex::new(s));
            app.manage(animation::ExportState::default());
            app.manage(captions::CaptionState::default());
            app.manage(sharing::ShareState::default());
            app.manage(spotlight::SpotlightState::default());
            let overlay = WebviewWindowBuilder::new(app,"overlay",WebviewUrl::App("index.html?view=overlay".into()))
                .data_directory(webview_data(app.handle())?)
                .title("Pointory · 필기").decorations(false).transparent(true).shadow(false).always_on_top(true)
                .skip_taskbar(true).visible(false).focused(false).resizable(false).build()?;
            WebviewWindowBuilder::new(app,"toolbar",WebviewUrl::App("index.html?view=toolbar".into()))
                .parent(&overlay)?
                .data_directory(webview_data(app.handle())?)
                .title("Pointory").inner_size(660.,64.).decorations(false).transparent(true).shadow(false)
                .always_on_top(true).resizable(false).focused(false).build()?;
            WebviewWindowBuilder::new(app,"settings",WebviewUrl::App("index.html?view=settings".into()))
                .parent(&overlay)?.always_on_top(true)
                .data_directory(webview_data(app.handle())?)
                .title("Pointory · 설정").inner_size(520.,700.).decorations(false).resizable(false).skip_taskbar(true).visible(false).build()?;
            let state = app.state::<Shared>();
            let mut s = state.lock().map_err(|e| e.to_string())?;
            #[cfg(target_os="linux")]
            if std::env::var("XDG_SESSION_TYPE").unwrap_or_default() == "wayland" {
                s.warning = Some("이 버전은 Linux X11 세션을 지원 대상으로 합니다. Wayland에서는 창 배치와 단축키가 제한될 수 있습니다. X11 세션에서 실행하세요.".into());
            }
            if s.prefs.global_shortcut_enabled {if let Err(e) = app.global_shortcut().register(s.prefs.shortcut.as_str()) { s.warning = Some(format!("단축키를 등록하지 못했습니다. 도구막대를 사용하세요: {e}")); }}
            if let Err(e) = save(app.handle(), &s.prefs) { s.warning = Some(format!("설정을 저장하지 못했습니다: {e}")); }
            if let Err(e) = place(app.handle(), &mut s) { s.warning = Some(e); let _ = show_settings(app.handle()); }
            publish(app.handle(), &s)?;
            drop(s);
            if std::env::var("POINTORY_CAPTIONS_OPEN").or_else(|_|std::env::var("ONPEN_CAPTIONS_OPEN")).or_else(|_|std::env::var("LAYERPEN_CAPTIONS_OPEN")).as_deref() == Ok("1") {
                let caption_app = app.handle().clone();
                tauri::async_runtime::spawn(async move { let _ = captions::caption_open(caption_app).await; });
            }
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_secs(2));
                let app = handle.clone();
                if handle.run_on_main_thread(move || {
                    if let Ok(current) = displays(&app) {
                        let state = app.state::<Shared>();
                        if let Ok(mut s) = state.lock() {
                            let now=now_ms();for items in s.fading.values_mut(){items.retain(|f|now<f.born+f.life);}
                            if current != s.displays {
                                s.displays = current;
                                if let Err(e) = place(&app,&mut s) { s.warning=Some(e); }
                                let _ = publish(&app,&s);
                            }
                        };
                    }
                }).is_err() { break; }
            });
            Ok(())
        })
        .on_window_event(|window,event| {
            if window.label()=="toolbar" && matches!(event,tauri::WindowEvent::Moved(_)|tauri::WindowEvent::Resized(_)|tauri::WindowEvent::ScaleFactorChanged{..}) {
                if window.app_handle().get_webview_window("settings").is_some_and(|w|w.is_visible().unwrap_or(false)){let _=dock_settings(window.app_handle());}
            }
            if let tauri::WindowEvent::CloseRequested {api,..} = event {
                match window.label() {
                    "settings" => { api.prevent_close(); let _ = window.hide(); }
                    "caption-settings" => { api.prevent_close(); captions::stop(window.app_handle()); let _ = window.hide(); }
                    "toolbar" => { captions::stop(window.app_handle()); window.app_handle().exit(0); },
                    _ => {}
                }
            }
        })
        .run(tauri::generate_context!()).expect("Pointory could not start");
}
