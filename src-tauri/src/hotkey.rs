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

use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{CloseHandle, ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_DWORD, REG_EXPAND_SZ, REG_OPTION_NON_VOLATILE,
    REG_SZ,
};
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

struct Key(HKEY);

impl Drop for Key {
    fn drop(&mut self) {
        unsafe { RegCloseKey(self.0) };
    }
}

fn open_key(path: &str, write: bool) -> Result<Key, String> {
    let mut hkey: HKEY = std::ptr::null_mut();
    let access = if write {
        KEY_READ | KEY_WRITE
    } else {
        KEY_READ
    };
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            wide(path).as_ptr(),
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

/// Current value, or an empty string when absent or unreadable.
fn read_disabled() -> String {
    let Ok(key) = open_key(KEY, false) else {
        return String::new();
    };
    let name = wide(VALUE);
    let mut kind = 0u32;
    let mut size = 0u32;
    // First the size, then the data: the value has no fixed length.
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            name.as_ptr(),
            std::ptr::null(),
            &mut kind,
            std::ptr::null_mut(),
            &mut size,
        )
    };
    if status != ERROR_SUCCESS || (kind != REG_SZ && kind != REG_EXPAND_SZ) {
        return String::new();
    }
    let mut buf = vec![0u16; (size as usize).div_ceil(2)];
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            name.as_ptr(),
            std::ptr::null(),
            &mut kind,
            buf.as_mut_ptr() as *mut u8,
            &mut size,
        )
    };
    if status != ERROR_SUCCESS {
        return String::new();
    }
    buf.truncate(size as usize / 2);
    String::from_utf16_lossy(&buf)
        .trim_end_matches('\0')
        .to_string()
}

/// Remember (or forget) that Clipper took Win+V over.
fn set_marker(on: bool) {
    let Ok(key) = open_key(MARKER_KEY, true) else {
        return;
    };
    let name = wide(MARKER);
    unsafe {
        if on {
            let one: u32 = 1;
            RegSetValueExW(
                key.0,
                name.as_ptr(),
                0,
                REG_DWORD,
                (&one as *const u32).cast(),
                4,
            );
        } else {
            RegDeleteValueW(key.0, name.as_ptr());
        }
    }
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
    let key = open_key(KEY, true)?;
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
    if status != ERROR_SUCCESS && !(letters.is_empty() && status == ERROR_FILE_NOT_FOUND) {
        return Err(format!(
            "Écriture dans le registre impossible (erreur {status})."
        ));
    }
    set_marker(disabled);
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
