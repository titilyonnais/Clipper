//! The two windows: `main` (full manager, in the taskbar when open) and
//! `popup` (quick paste palette, created hidden at start-up so it appears
//! instantly, centred on the screen that holds the mouse pointer).

use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

pub const MAIN: &str = "main";
pub const POPUP: &str = "popup";
/// The popup window is transparent: the panel (820 x 520) is drawn by the
/// interface with its own corners and shadow, inside a 32 px margin, so the
/// whole panel can fade in and out.
const POPUP_SIZE: (f64, f64) = (820.0 + 64.0, 520.0 + 64.0);

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
        .shadow(false)
        .transparent(true)
        .drag_and_drop(false)
        .build()
        .inspect(disable_transitions)
}

/// The popup plays its own entrance animation; Windows' generic one would
/// run on top of it.
fn disable_transitions(w: &WebviewWindow) {
    use windows_sys::Win32::Graphics::Dwm::DWMWA_TRANSITIONS_FORCEDISABLED;
    if let Ok(hwnd) = w.hwnd() {
        let hwnd = hwnd.0 as windows_sys::Win32::Foundation::HWND;
        set_dwm_attribute(hwnd, DWMWA_TRANSITIONS_FORCEDISABLED, &1i32);
    }
}

/// Windows 11 draws a thin light frame around undecorated windows, brightest
/// along the top edge. Remove it, keep the rounded corners, and give the
/// non-client area the colour of the theme so nothing shows on the edges.
pub fn style_frame(w: &WebviewWindow, light: bool) {
    use windows_sys::Win32::Graphics::Dwm::{
        DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_COLOR_NONE, DWMWA_USE_IMMERSIVE_DARK_MODE,
        DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    };
    let Ok(hwnd) = w.hwnd() else {
        return;
    };
    let hwnd = hwnd.0 as windows_sys::Win32::Foundation::HWND;
    let caption: u32 = if light { 0x00FF_FFFF } else { 0 };
    // Best effort: Windows 10 only knows the first attribute.
    set_dwm_attribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &i32::from(!light));
    set_dwm_attribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &DWMWCP_ROUND);
    set_dwm_attribute(hwnd, DWMWA_BORDER_COLOR, &DWMWA_COLOR_NONE);
    set_dwm_attribute(hwnd, DWMWA_CAPTION_COLOR, &caption);
}

fn set_dwm_attribute<T>(hwnd: windows_sys::Win32::Foundation::HWND, attribute: i32, value: &T) {
    unsafe {
        windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute(
            hwnd,
            attribute as u32,
            (value as *const T).cast(),
            std::mem::size_of::<T>() as u32,
        );
    }
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
