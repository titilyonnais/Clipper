//! Win+V takeover.
//!
//! Explorer reserves Win+V for Windows' clipboard history. The supported way
//! to free it is the per-user `DisabledHotkeys` value (letters Explorer must
//! not register), read when Explorer starts. Clipper edits that value, offers
//! to restart Explorer gracefully, then registers Win+V like any global
//! shortcut. No keyboard hook is involved.

use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW};

const KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
const VALUE: &str = "DisabledHotkeys";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

struct Key(HKEY);

impl Drop for Key {
    fn drop(&mut self) {
        unsafe { RegCloseKey(self.0) };
    }
}

fn open_key(write: bool) -> Result<Key, String> {
    let mut hkey: HKEY = std::ptr::null_mut();
    let access = if write {
        KEY_READ | KEY_WRITE
    } else {
        KEY_READ
    };
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            wide(KEY).as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            access,
            std::ptr::null(),
            &mut hkey,
            std::ptr::null_mut(),
        )
    };
    if status != ERROR_SUCCESS {
        return Err(format!("Registre inaccessible (erreur {status})."));
    }
    Ok(Key(hkey))
}

fn read_disabled() -> String {
    let Ok(key) = open_key(false) else {
        return String::new();
    };
    let mut buf = [0u16; 256];
    let mut size = (buf.len() * 2) as u32;
    let mut kind = 0u32;
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            wide(VALUE).as_ptr(),
            std::ptr::null(),
            &mut kind,
            buf.as_mut_ptr() as *mut u8,
            &mut size,
        )
    };
    if status != ERROR_SUCCESS || kind != REG_SZ {
        return String::new();
    }
    let len = (size as usize / 2).min(buf.len());
    String::from_utf16_lossy(&buf[..len])
        .trim_end_matches('\0')
        .to_string()
}

/// Add or remove "V" from Explorer's disabled hotkeys (other letters are kept).
pub fn set_explorer_win_v_disabled(disabled: bool) -> Result<(), String> {
    let current = read_disabled();
    let mut letters: String = current
        .chars()
        .filter(|c| !c.eq_ignore_ascii_case(&'v'))
        .collect();
    if disabled {
        letters.push('V');
    }
    let key = open_key(true)?;
    let status = if letters.is_empty() {
        unsafe { RegDeleteValueW(key.0, wide(VALUE).as_ptr()) }
    } else {
        let data = wide(&letters);
        unsafe {
            RegSetValueExW(
                key.0,
                wide(VALUE).as_ptr(),
                0,
                REG_SZ,
                data.as_ptr() as *const u8,
                (data.len() * 2) as u32,
            )
        }
    };
    if status != ERROR_SUCCESS && !(letters.is_empty() && status == 2) {
        return Err(format!(
            "Écriture dans le registre impossible (erreur {status})."
        ));
    }
    Ok(())
}

/// Restart Explorer so it reloads its hotkeys. Uses the same message as
/// "Exit Explorer" in the taskbar context menu, then starts it again.
pub fn restart_explorer() -> Result<(), String> {
    let class = wide("Shell_TrayWnd");
    let tray = unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) };
    if !tray.is_null() {
        unsafe { PostMessageW(tray, 0x5B4, 0, 0) };
        let deadline = Instant::now() + Duration::from_secs(8);
        while !unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) }.is_null() {
            if Instant::now() > deadline {
                return Err("L'Explorateur ne s'est pas fermé. Déconnectez-vous puis reconnectez-vous pour appliquer Win+V.".into());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        // Give the process time to exit completely.
        std::thread::sleep(Duration::from_millis(500));
    }
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
    std::process::Command::new(format!(r"{windir}\explorer.exe"))
        .spawn()
        .map_err(|e| format!("Relance de l'Explorateur impossible : {e}"))?;
    Ok(())
}
