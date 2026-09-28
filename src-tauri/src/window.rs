//! The two windows: `main` (full manager, in the taskbar when open) and
//! `popup` (quick paste palette, created hidden at start-up so it appears
//! instantly, centred on the screen that holds the mouse pointer).

use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

pub const MAIN: &str = "main";
pub const POPUP: &str = "popup";
const POPUP_SIZE: (f64, f64) = (820.0, 520.0);

pub fn create_popup(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    WebviewWindowBuilder::new(app, POPUP, WebviewUrl::App("index.html".into()))
        .title("Clipper")
        .inner_size(POPUP_SIZE.0, POPUP_SIZE.1)
        .decorations(false)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .focused(false)
        .shadow(true)
        .drag_and_drop(false)
        .build()
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(MAIN) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        let _ = w.emit("window:shown", ());
    }
}

pub fn hide_popup(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(POPUP) {
        let _ = w.hide();
    }
}

pub fn show_popup(app: &AppHandle) {
    let Some(w) = app.get_webview_window(POPUP) else {
        return;
    };
    crate::paste::remember_target();
    center_on_pointer_screen(app, &w);
    let _ = w.show();
    let _ = w.set_focus();
    let _ = w.emit("popup:shown", crate::paste::target_app());
}

pub fn toggle_popup(app: &AppHandle) {
    let Some(w) = app.get_webview_window(POPUP) else {
        return;
    };
    if w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false) {
        let _ = w.hide();
    } else {
        show_popup(app);
    }
}

fn center_on_pointer_screen(app: &AppHandle, w: &WebviewWindow) {
    let monitors = app.available_monitors().unwrap_or_default();
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| {
            monitors.iter().find(|m| {
                let (pos, size) = (m.position(), m.size());
                p.x >= pos.x as f64
                    && p.y >= pos.y as f64
                    && p.x < (pos.x + size.width as i32) as f64
                    && p.y < (pos.y + size.height as i32) as f64
            })
        })
        .cloned()
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(m) = monitor else {
        let _ = w.center();
        return;
    };
    let scale = m.scale_factor();
    let (width, height) = ((POPUP_SIZE.0 * scale) as i32, (POPUP_SIZE.1 * scale) as i32);
    let work = m.work_area();
    let x = work.position.x + (work.size.width as i32 - width) / 2;
    // Slightly above the centre, where the eye naturally goes.
    let y = work.position.y + ((work.size.height as i32 - height) as f64 * 0.4) as i32;
    let _ = w.set_position(PhysicalPosition::new(x, y));
}
