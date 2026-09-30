//! The two windows: `main` (full manager, in the taskbar when open) and
//! `popup` (quick paste palette, created hidden at start-up so it appears
//! instantly, centred on the screen that holds the mouse pointer).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

pub const MAIN: &str = "main";
pub const POPUP: &str = "popup";
/// Size of the popup. Its window is opaque, with the rounded corners and
/// shadow Windows gives its own popups: a transparent window let a light
/// outline show around the panel while it animated.
const POPUP_SIZE: (f64, f64) = (820.0, 520.0);

/// The main window, as described in `tauri.conf.json`. It is created by
/// Clipper rather than at start-up so the web view's folder can be tidied
/// before the engine opens it (see `webcache`).
pub fn create_main(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    WebviewWindowBuilder::from_config(app, main_config(app))?.build()
}

fn main_config(app: &AppHandle) -> &tauri::utils::config::WindowConfig {
    let windows = &app.config().app.windows;
    windows
        .iter()
        .find(|w| w.label == MAIN)
        .unwrap_or(&windows[0])
}

pub fn create_popup(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let mut builder = WebviewWindowBuilder::new(app, POPUP, WebviewUrl::App("index.html".into()));
    // Both windows share one engine, which requires the same arguments.
    if let Some(args) = &main_config(app).additional_browser_args {
        builder = builder.additional_browser_args(args);
    }
    builder
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
        .background_color(tauri::window::Color(0, 0, 0, 255))
        .drag_and_drop(false)
        .build()
        .inspect(|w| style_frame(w, false))
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

/// Incremented at each appearance of the popup.
static POPUP_SHOWN: AtomicU64 = AtomicU64::new(0);

pub fn hide_popup(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(POPUP) {
        let _ = w.hide();
    }
}

/// Hide the popup through its interface, which first presents an empty
/// frame (see `PopupApp`): Windows shows a window's last frame when it
/// appears again. If the interface does not answer, hide it anyway.
pub fn dismiss_popup(app: &AppHandle) {
    let Some(w) = app.get_webview_window(POPUP) else {
        return;
    };
    if !w.is_visible().unwrap_or(false) {
        return;
    }
    let _ = w.emit("popup:dismiss", ());
    let shown = POPUP_SHOWN.load(Ordering::SeqCst);
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(150));
        // Not if it was shown again meanwhile.
        if POPUP_SHOWN.load(Ordering::SeqCst) == shown {
            hide_popup(&app);
        }
    });
}

pub fn show_popup(app: &AppHandle) {
    let Some(w) = app.get_webview_window(POPUP) else {
        return;
    };
    crate::paste::remember_target();
    POPUP_SHOWN.fetch_add(1, Ordering::SeqCst);
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
        dismiss_popup(app);
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
