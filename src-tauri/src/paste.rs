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
    GetForegroundWindow, GetWindowThreadProcessId, IsWindow, SetForegroundWindow,
};

const VK_V: VIRTUAL_KEY = 0x56;

static TARGET: AtomicIsize = AtomicIsize::new(0);

/// Remember the foreground window (call just before showing the popup).
/// Opened from Clipper itself, there is no target: the popup only copies,
/// rather than pasting into a window used long ago.
pub fn remember_target() {
    let hwnd = unsafe { GetForegroundWindow() };
    let is_clipper =
        crate::clipboard::process_of_window(hwnd).is_some_and(|(pid, _)| pid == std::process::id());
    let target = if is_clipper {
        std::ptr::null_mut()
    } else {
        hwnd
    };
    TARGET.store(target as isize, Ordering::SeqCst);
}

fn target() -> Option<HWND> {
    let hwnd = TARGET.load(Ordering::SeqCst) as HWND;
    (!hwnd.is_null() && unsafe { IsWindow(hwnd) } != 0).then_some(hwnd)
}

/// Whether a paste can be attempted: `Ok(false)` when there is no target
/// (the item is simply copied), an error when Windows would block the keys
/// (UIPI: the target runs as administrator and Clipper does not).
pub fn can_paste() -> Result<bool, String> {
    let Some(hwnd) = target() else {
        return Ok(false);
    };
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    if process_elevated(pid) && !process_elevated(std::process::id()) {
        return Err("Cette application est lancée en administrateur : Windows empêche Clipper d'y coller. L'élément est copié, collez-le avec Ctrl+V.".into());
    }
    Ok(true)
}

fn process_elevated(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return false;
        }
        let mut token = std::ptr::null_mut();
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut len = 0u32;
        let ok = OpenProcessToken(process, TOKEN_QUERY, &mut token) != 0
            && GetTokenInformation(
                token,
                TokenElevation,
                (&mut elevation as *mut TOKEN_ELEVATION).cast(),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut len,
            ) != 0;
        if !token.is_null() {
            CloseHandle(token);
        }
        CloseHandle(process);
        ok && elevation.TokenIsElevated != 0
    }
}

/// Executable name of the remembered window, for display ("Coller dans …").
pub fn target_app() -> Option<String> {
    crate::clipboard::process_of_window(target()?).and_then(|(_, path)| {
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
    let hwnd = target().ok_or("Aucune application où coller.")?;
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
        return Err("Collage bloqué par Windows. L'élément est dans le presse-papiers.".into());
    }
    Ok(())
}
