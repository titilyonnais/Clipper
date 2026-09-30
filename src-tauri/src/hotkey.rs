//! Win+V takeover.
//!
//! Explorer reserves Win+V for Windows' clipboard history. The supported way
//! to free it is the per-user `DisabledHotkeys` value (letters Explorer must
//! not register), read when Explorer starts. Clipper edits that value, offers
//! to restart Explorer gracefully, then registers Win+V like any global
//! shortcut. No keyboard hook is involved.
//!
//! A marker (`HKCU\Software\Clipper\WinVTakenOver`) records that the letter
//! was added by Clipper, so the uninstaller gives Win+V back to Windows.

use crate::registry::Key;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::System::Threading::{
    OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetWindowThreadProcessId, PostMessageW,
};

const KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
const VALUE: &str = "DisabledHotkeys";
const MARKER_KEY: &str = r"Software\Clipper";
const MARKER: &str = "WinVTakenOver";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Add or remove "V" from Explorer's disabled hotkeys (other letters are kept).
pub fn set_explorer_win_v_disabled(disabled: bool) -> Result<(), String> {
    let key = Key::open(KEY, true)?;
    let mut letters: String = key
        .string(VALUE)
        .unwrap_or_default()
        .chars()
        .filter(|c| !c.eq_ignore_ascii_case(&'v'))
        .collect();
    if disabled {
        letters.push('V');
    }
    if letters.is_empty() {
        key.delete(VALUE)?;
    } else {
        key.set_string(VALUE, &letters)?;
    }
    // Remember (or forget) that Clipper took Win+V over, for the uninstaller.
    if let Ok(marker) = Key::open(MARKER_KEY, true) {
        let _ = if disabled {
            marker.set_dword(MARKER, 1)
        } else {
            marker.delete(MARKER)
        };
    }
    Ok(())
}

/// Restart Explorer so it reloads its hotkeys. Uses the same message as
/// "Exit Explorer" in the taskbar context menu, then starts it again.
pub fn restart_explorer() -> Result<(), String> {
    let class = wide("Shell_TrayWnd");
    let tray = unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) };
    if !tray.is_null() {
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(tray, &mut pid) };
        let process = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
        unsafe { PostMessageW(tray, 0x5B4, 0, 0) };
        let deadline = Instant::now() + Duration::from_secs(8);
        while !unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) }.is_null() {
            if Instant::now() > deadline {
                if !process.is_null() {
                    unsafe { CloseHandle(process) };
                }
                return Err("L'Explorateur ne s'est pas fermé. Déconnectez-vous puis reconnectez-vous pour appliquer Win+V.".into());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        // Starting explorer.exe while the old shell still runs would only
        // open a folder window: wait for the process itself to end.
        if !process.is_null() {
            unsafe {
                WaitForSingleObject(process, 5000);
                CloseHandle(process);
            }
        }
    }
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
    std::process::Command::new(format!(r"{windir}\explorer.exe"))
        .spawn()
        .map_err(|e| format!("Relance de l'Explorateur impossible : {e}"))?;
    Ok(())
}
