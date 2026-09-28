//! Paste into the application that was active before the popup opened: its
//! window gets the focus back, then Clipper sends Ctrl+V (`SendInput`).
//!
//! Windows blocks input sent to applications running as administrator from a
//! normal process (UIPI); the item then simply stays on the clipboard.

use std::sync::atomic::{AtomicIsize, Ordering};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
    VK_CONTROL, VK_LSHIFT, VK_RSHIFT, VK_SHIFT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, IsWindow, SetForegroundWindow,
};

const VK_V: VIRTUAL_KEY = 0x56;

static TARGET: AtomicIsize = AtomicIsize::new(0);

/// Remember the foreground window (call just before showing the popup).
pub fn remember_target() {
    let hwnd = unsafe { GetForegroundWindow() };
    let is_clipper =
        crate::clipboard::process_of_window(hwnd).is_some_and(|(pid, _)| pid == std::process::id());
    if !is_clipper {
        TARGET.store(hwnd as isize, Ordering::SeqCst);
    }
}

/// Executable name of the remembered window, for display ("Coller dans …").
pub fn target_app() -> Option<String> {
    let hwnd = TARGET.load(Ordering::SeqCst) as HWND;
    if hwnd.is_null() || unsafe { IsWindow(hwnd) } == 0 {
        return None;
    }
    crate::clipboard::process_of_window(hwnd).and_then(|(_, path)| {
        std::path::Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
    })
}

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up { KEYEVENTF_KEYUP } else { 0 },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// Focus the remembered window and send Ctrl+V. Must be called while a
/// Clipper window still has the focus (Windows only lets the foreground
/// process hand the focus over). `shift_held` releases Shift first so that
/// Maj+Entrée does not turn into Ctrl+Maj+V in the target.
pub fn paste_into_target(shift_held: bool) -> Result<(), String> {
    let hwnd = TARGET.load(Ordering::SeqCst) as HWND;
    if hwnd.is_null() || unsafe { IsWindow(hwnd) } == 0 {
        return Err("Aucune application où coller.".into());
    }
    unsafe { SetForegroundWindow(hwnd) };
    let deadline = Instant::now() + Duration::from_millis(400);
    while unsafe { GetForegroundWindow() } != hwnd {
        if Instant::now() > deadline {
            return Err("L'application cible n'a pas repris le focus.".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    // Let the target finish activating before it receives the keys.
    std::thread::sleep(Duration::from_millis(40));
    let mut inputs = Vec::with_capacity(7);
    if shift_held {
        inputs.extend([
            key(VK_SHIFT, true),
            key(VK_LSHIFT, true),
            key(VK_RSHIFT, true),
        ]);
    }
    inputs.extend([
        key(VK_CONTROL, false),
        key(VK_V, false),
        key(VK_V, true),
        key(VK_CONTROL, true),
    ]);
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    if sent as usize != inputs.len() {
        return Err("Collage bloqué par Windows (application lancée en administrateur ?). L'élément est dans le presse-papiers.".into());
    }
    Ok(())
}
