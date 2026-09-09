//! Native macOS placement for the annotation overlay; other platforms keep Tauri placement.
use crate::{model::Display, Result};
use tauri::WebviewWindow;

/// Keep controls above the annotation surface and the pointer spotlight.
pub(crate) fn set_level(window: &WebviewWindow) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let offset = match window.label() {
            "overlay" => 1,
            "spotlight" => 2,
            "captions" => 3,
            "toolbar" => 4,
            _ => 5,
        };
        on_main(window, move |native| {
            native.setLevel(objc2_app_kit::NSMainMenuWindowLevel + offset);
            Ok(())
        })
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = window; Ok(()) }
}

/// A low-level borderless NSWindow is clamped below the menu bar by AppKit.
/// Raise the overlay before setting its complete logical frame in one operation.
pub(crate) fn place_overlay(window: &WebviewWindow, display: &Display) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let display = display.clone();
        on_main(window, move |native| {
            use objc2::MainThreadMarker;
            use objc2_app_kit::{NSScreen, NSMainMenuWindowLevel};
            use objc2_foundation::{NSPoint, NSRect, NSSize};
            let marker = MainThreadMarker::new().ok_or("Window placement requires the main thread")?;
            // NSScreen.screens starts with the display containing the menu bar.
            let screens = NSScreen::screens(marker);
            let primary = screens.firstObject().ok_or("Primary screen unavailable")?;
            let primary_frame = primary.frame();
            let (x, y, width, height) = appkit_frame(&display,
                primary_frame.origin.y + primary_frame.size.height);
            native.setLevel(NSMainMenuWindowLevel + 1);
            native.setFrame_display_animate(
                NSRect::new(NSPoint::new(x, y), NSSize::new(width, height)), true, false);
            Ok(())
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        window.set_position(tauri::PhysicalPosition::new(display.x, display.y)).map_err(|e| e.to_string())?;
        window.set_size(tauri::PhysicalSize::new(display.width, display.height)).map_err(|e| e.to_string())
    }
}

#[cfg(any(target_os = "macos", test))]
fn appkit_frame(display: &Display, primary_top: f64) -> (f64, f64, f64, f64) {
    let width = f64::from(display.width) / display.scale;
    let height = f64::from(display.height) / display.scale;
    (f64::from(display.x) / display.scale,
        primary_top - f64::from(display.y) / display.scale - height, width, height)
}

#[cfg(target_os = "macos")]
fn on_main<F>(window: &WebviewWindow, operation: F) -> Result<()>
where F: FnOnce(&objc2_app_kit::NSWindow) -> Result<()> + Send + 'static {
    fn apply<F>(window: &WebviewWindow, operation: F) -> Result<()>
    where F: FnOnce(&objc2_app_kit::NSWindow) -> Result<()> {
        let pointer = window.ns_window().map_err(|e| e.to_string())?;
        // Tauri owns the NSWindow; the live WebviewWindow keeps it alive for this
        // synchronous main-thread operation. No subclass or global swizzle is used.
        let native = unsafe { (pointer as *const objc2_app_kit::NSWindow).as_ref() }
            .ok_or("Native window unavailable")?;
        operation(native)
    }
    if objc2::MainThreadMarker::new().is_some() { return apply(window, operation); }
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let handle = window.clone();
    window.run_on_main_thread(move || { let _ = sender.send(apply(&handle, operation)); })
        .map_err(|e| e.to_string())?;
    receiver.recv().map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    fn display(x: i32, y: i32, width: u32, height: u32, scale: f64) -> Display {
        Display { id: "test".into(), name: "test".into(), x, y, width, height, scale }
    }
    #[test]
    fn overlay_frame_covers_the_entire_screen_including_menu_bar() {
        assert_eq!(appkit_frame(&display(0, 0, 1920, 1080, 1.), 1080.), (0., 0., 1920., 1080.));
        assert_eq!(appkit_frame(&display(0, 0, 2880, 1800, 2.), 900.), (0., 0., 1440., 900.));
    }
    #[test]
    fn overlay_frame_handles_displays_left_and_above_the_primary() {
        assert_eq!(appkit_frame(&display(-2880, -1800, 2880, 1800, 2.), 900.), (-1440., 900., 1440., 900.));
    }
}
