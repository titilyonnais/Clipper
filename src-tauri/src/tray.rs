//! Notification-area icon and incognito mode.
//!
//! The icon is monochrome and follows the taskbar theme (Windows can use a
//! dark taskbar with light apps); a crossed-out variant shows that capture is
//! paused.

use crate::commands::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegGetValueW, RegNotifyChangeKeyValue, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER,
    KEY_NOTIFY, REG_NOTIFY_CHANGE_LAST_SET, RRF_RT_REG_DWORD,
};

const TRAY_ID: &str = "main";
const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";

static LIGHT_TASKBAR: AtomicBool = AtomicBool::new(false);

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn taskbar_is_light() -> bool {
    let mut value: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            wide(PERSONALIZE).as_ptr(),
            wide("SystemUsesLightTheme").as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            &mut value as *mut u32 as *mut _,
            &mut size,
        )
    };
    status == ERROR_SUCCESS && value == 1
}

fn icon(paused: bool) -> Image<'static> {
    let bytes: &'static [u8] = match (LIGHT_TASKBAR.load(Ordering::Relaxed), paused) {
        (false, false) => include_bytes!("../icons/tray/tray-white.png"),
        (false, true) => include_bytes!("../icons/tray/tray-white-paused.png"),
        (true, false) => include_bytes!("../icons/tray/tray-black.png"),
        (true, true) => include_bytes!("../icons/tray/tray-black-paused.png"),
    };
    Image::from_bytes(bytes).expect("valid embedded tray icon")
}

fn is_paused(app: &AppHandle) -> bool {
    app.try_state::<AppState>()
        .is_some_and(|s| s.monitor.is_paused())
}

fn build_menu(app: &AppHandle, paused: bool) -> tauri::Result<Menu<tauri::Wry>> {
    let open = MenuItem::with_id(app, "open", "Ouvrir Clipper", true, None::<&str>)?;
    let popup = MenuItem::with_id(app, "popup", "Collage rapide", true, None::<&str>)?;
    let resume = MenuItem::with_id(app, "resume", "Reprendre la capture", paused, None::<&str>)?;
    let incognito = Submenu::with_items(
        app,
        "Mode incognito",
        true,
        &[
            &MenuItem::with_id(app, "pause-5", "Pendant 5 minutes", true, None::<&str>)?,
            &MenuItem::with_id(app, "pause-60", "Pendant 1 heure", true, None::<&str>)?,
            &MenuItem::with_id(app, "pause-0", "Jusqu'à réactivation", true, None::<&str>)?,
        ],
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quitter Clipper", true, None::<&str>)?;
    let sep = || PredefinedMenuItem::separator(app);
    Menu::with_items(
        app,
        &[&open, &popup, &sep()?, &incognito, &resume, &sep()?, &quit],
    )
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    LIGHT_TASKBAR.store(taskbar_is_light(), Ordering::Relaxed);
    let paused = is_paused(app);
    TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(tooltip(paused))
        .icon(icon(paused))
        .menu(&build_menu(app, paused)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => crate::window::show_main(app),
            "popup" => crate::window::show_popup(app),
            "resume" => set_paused(app, false, None),
            "pause-5" => set_paused(app, true, Some(Duration::from_secs(5 * 60))),
            "pause-60" => set_paused(app, true, Some(Duration::from_secs(60 * 60))),
            "pause-0" => set_paused(app, true, None),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                crate::window::show_main(tray.app_handle());
            }
        })
        .build(app)?;
    watch_taskbar_theme(app.clone());
    Ok(())
}

fn tooltip(paused: bool) -> &'static str {
    if paused {
        "Clipper · capture suspendue"
    } else {
        "Clipper"
    }
}

/// Refresh icon, tooltip and menu after a state change.
pub fn refresh(app: &AppHandle) {
    let paused = is_paused(app);
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(icon(paused)));
        let _ = tray.set_tooltip(Some(tooltip(paused)));
        if let Ok(menu) = build_menu(app, paused) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

/// Pause (incognito) or resume capture, everywhere at once. A pause "until
/// resumed" is remembered across restarts; a timed one is not.
pub fn set_paused(app: &AppHandle, paused: bool, duration: Option<Duration>) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    if paused {
        state.monitor.pause(duration);
    } else {
        state.monitor.resume();
    }
    if let Ok(mut s) = state.db.get_settings() {
        let forever = paused && duration.is_none();
        if s.monitor_paused != forever {
            s.monitor_paused = forever;
            let _ = state.db.set_settings(&s);
        }
    }
    let _ = app.emit("monitor:paused", state.monitor.paused_until());
    refresh(app);
    if let (true, Some(d)) = (paused, duration) {
        let app = app.clone();
        let until = state.monitor.paused_until();
        std::thread::spawn(move || {
            std::thread::sleep(d + Duration::from_secs(1));
            let Some(state) = app.try_state::<AppState>() else {
                return;
            };
            // Only end this pause, not one started later.
            if state.monitor.paused_until() == until || !state.monitor.is_paused() {
                state.monitor.resume();
                let _ = app.emit("monitor:paused", None::<String>);
                refresh(&app);
            }
        });
    }
}

/// Follow taskbar theme changes without polling.
fn watch_taskbar_theme(app: AppHandle) {
    std::thread::Builder::new()
        .name("taskbar-theme".into())
        .spawn(move || unsafe {
            let mut key: HKEY = std::ptr::null_mut();
            if RegOpenKeyExW(
                HKEY_CURRENT_USER,
                wide(PERSONALIZE).as_ptr(),
                0,
                KEY_NOTIFY,
                &mut key,
            ) != ERROR_SUCCESS
            {
                return;
            }
            loop {
                // Blocks until a value under the key changes.
                if RegNotifyChangeKeyValue(
                    key,
                    0,
                    REG_NOTIFY_CHANGE_LAST_SET,
                    std::ptr::null_mut(),
                    0,
                ) != ERROR_SUCCESS
                {
                    break;
                }
                let light = taskbar_is_light();
                if LIGHT_TASKBAR.swap(light, Ordering::Relaxed) != light {
                    refresh(&app);
                }
            }
            RegCloseKey(key);
        })
        .expect("failed to start theme watcher");
}
