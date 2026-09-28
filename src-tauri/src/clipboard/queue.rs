//! Paste queue: each paste in any application delivers the next item.
//!
//! Clipper announces text on the clipboard without providing it (delayed
//! rendering). When an application pastes, Windows asks Clipper for the data
//! (`WM_RENDERFORMAT`); Clipper hands over the current item and immediately
//! announces the next one. No keyboard monitoring is involved.

use parking_lot::Mutex;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{GlobalFree, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardOwner, GetOpenClipboardWindow, OpenClipboard,
    SetClipboardData,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE, GMEM_ZEROINIT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetForegroundWindow, GetMessageW,
    GetWindowThreadProcessId, PostMessageW, RegisterClassW, SendMessageTimeoutW, HWND_MESSAGE, MSG,
    SMTO_ABORTIFHUNG, WM_APP, WNDCLASSW,
};

const CF_UNICODETEXT: u32 = 13;
const WM_RENDERFORMAT: u32 = 0x0305;
const WM_RENDERALLFORMATS: u32 = 0x0306;
const WM_DESTROYCLIPBOARD: u32 = 0x0307;
const WM_TAKE: u32 = WM_APP + 1;
const WM_STOP: u32 = WM_APP + 2;

#[derive(Debug, Clone, Default, Serialize)]
pub struct QueueStatus {
    pub active: bool,
    /// Index of the next item to paste.
    pub position: usize,
    pub total: usize,
}

#[derive(Default)]
struct State {
    items: Vec<String>,
    position: usize,
    active: bool,
}

static STATE: Mutex<State> = Mutex::new(State {
    items: Vec::new(),
    position: 0,
    active: false,
});
static WINDOW: AtomicIsize = AtomicIsize::new(0);
/// Set while Clipper empties the clipboard itself, so that the resulting
/// `WM_DESTROYCLIPBOARD` is not mistaken for another application copying.
static RETAKING: AtomicBool = AtomicBool::new(false);
static NOTIFY: OnceLock<Box<dyn Fn(QueueStatus) + Send + Sync>> = OnceLock::new();
/// When the current item was announced: background monitors read within a
/// few milliseconds of a clipboard change, a person pastes much later.
static ANNOUNCED: Mutex<Option<Instant>> = Mutex::new(None);
const HUMAN_DELAY: Duration = Duration::from_millis(250);

pub fn status() -> QueueStatus {
    let s = STATE.lock();
    QueueStatus {
        active: s.active,
        position: s.position,
        total: s.items.len(),
    }
}

fn notify() {
    if let Some(f) = NOTIFY.get() {
        f(status());
    }
}

/// Create the hidden window that owns the clipboard while a queue runs.
pub fn init(on_change: impl Fn(QueueStatus) + Send + Sync + 'static) {
    let _ = NOTIFY.set(Box::new(on_change));
    std::thread::Builder::new()
        .name("paste-queue".into())
        .spawn(|| unsafe {
            let class: Vec<u16> = "ClipperPasteQueue\0".encode_utf16().collect();
            let instance = GetModuleHandleW(std::ptr::null());
            let wc = WNDCLASSW {
                lpfnWndProc: Some(wndproc),
                hInstance: instance,
                lpszClassName: class.as_ptr(),
                ..std::mem::zeroed()
            };
            RegisterClassW(&wc);
            let hwnd = CreateWindowExW(
                0,
                class.as_ptr(),
                class.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            );
            if hwnd.is_null() {
                log::error!("paste queue window could not be created");
                return;
            }
            WINDOW.store(hwnd as isize, Ordering::SeqCst);
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                DispatchMessageW(&msg);
            }
        })
        .expect("failed to start paste queue thread");
}

fn post(msg: u32) -> bool {
    let hwnd = WINDOW.load(Ordering::SeqCst) as HWND;
    !hwnd.is_null() && unsafe { PostMessageW(hwnd, msg, 0, 0) } != 0
}

/// Start pasting `items` one per paste.
pub fn start(items: Vec<String>) -> Result<(), String> {
    if items.is_empty() {
        return Err("Sélectionnez au moins un élément texte.".into());
    }
    {
        let mut s = STATE.lock();
        s.items = items;
        s.position = 0;
        s.active = true;
    }
    if !post(WM_TAKE) {
        STATE.lock().active = false;
        return Err("File de collage indisponible.".into());
    }
    notify();
    Ok(())
}

pub fn stop() {
    post(WM_STOP);
}

/// Stop and wait until the pending item is really on the clipboard (on
/// exit: the process must not end before the queue thread has done it).
pub fn stop_and_wait() {
    let hwnd = WINDOW.load(Ordering::SeqCst) as HWND;
    if !hwnd.is_null() {
        let mut result = 0usize;
        unsafe { SendMessageTimeoutW(hwnd, WM_STOP, 0, 0, SMTO_ABORTIFHUNG, 1000, &mut result) };
    }
}

unsafe fn open_clipboard(hwnd: HWND) -> bool {
    // The application that just pasted may still hold the clipboard.
    for _ in 0..50 {
        if OpenClipboard(hwnd) != 0 {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

/// Put `text` on the (already open) clipboard.
unsafe fn set_text(text: &str) {
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = wide.len() * 2;
    let handle = GlobalAlloc(GMEM_MOVEABLE, bytes);
    if handle.is_null() {
        return;
    }
    let ptr = GlobalLock(handle) as *mut u16;
    if ptr.is_null() {
        GlobalFree(handle);
        return;
    }
    std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
    GlobalUnlock(handle);
    // On success the clipboard owns the memory; otherwise it is ours to free.
    if SetClipboardData(CF_UNICODETEXT, handle).is_null() {
        GlobalFree(handle);
    }
}

/// Announce the current item without providing it yet.
unsafe fn take_ownership(hwnd: HWND) {
    if !STATE.lock().active {
        return;
    }
    if !open_clipboard(hwnd) {
        log::warn!("paste queue: clipboard busy");
        return;
    }
    RETAKING.store(true, Ordering::SeqCst);
    EmptyClipboard();
    RETAKING.store(false, Ordering::SeqCst);
    SetClipboardData(CF_UNICODETEXT, std::ptr::null_mut());
    *ANNOUNCED.lock() = Some(Instant::now());
    // Ask other clipboard managers (and Windows' history) not to read the
    // announced data: reading it would consume a queue item.
    for name in [
        "ExcludeClipboardContentFromMonitorProcessing",
        "Clipboard Viewer Ignore",
    ] {
        if let Some(fmt) = super::win::registered(name) {
            let handle = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, 4);
            if !handle.is_null() && SetClipboardData(fmt, handle).is_null() {
                GlobalFree(handle);
            }
        }
    }
    CloseClipboard();
}

/// Whether the application reading the clipboard is the one in front, i.e.
/// the user is pasting. Background readers (sync tools, other clipboard
/// managers) get the current item without consuming it.
unsafe fn read_by_foreground_app() -> bool {
    let reader = GetOpenClipboardWindow();
    if reader.is_null() {
        // Opened without a window: a paste only if it is not an immediate
        // reaction to the announcement.
        let late = ANNOUNCED.lock().is_none_or(|t| t.elapsed() >= HUMAN_DELAY);
        log::debug!("paste queue: read by a window-less reader (counted: {late})");
        return late;
    }
    let pid_of = |hwnd: HWND| {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        pid
    };
    let reader_pid = pid_of(reader);
    let counts = reader_pid != 0 && reader_pid == pid_of(GetForegroundWindow());
    log::debug!(
        "paste queue: read by {:?} (foreground: {counts})",
        super::win::process_of_window(reader).map(|(_, p)| p)
    );
    counts
}

fn current_item() -> Option<String> {
    let s = STATE.lock();
    s.items.get(s.position).cloned()
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_TAKE => {
            take_ownership(hwnd);
            0
        }
        WM_RENDERFORMAT => {
            if wparam as u32 == CF_UNICODETEXT {
                if let Some(text) = current_item() {
                    set_text(&text);
                }
                let pasted = read_by_foreground_app();
                let active = {
                    let mut s = STATE.lock();
                    if pasted {
                        s.position += 1;
                        s.active = s.position < s.items.len();
                    }
                    s.active
                };
                if pasted {
                    notify();
                }
                // Announce the next item (or the same one after a background read).
                if active {
                    PostMessageW(hwnd, WM_TAKE, 0, 0);
                }
            }
            0
        }
        WM_RENDERALLFORMATS => {
            // Clipper is exiting while owning announced data: provide it.
            if open_clipboard(hwnd) {
                if GetClipboardOwner() == hwnd {
                    if let Some(text) = current_item() {
                        set_text(&text);
                    }
                }
                CloseClipboard();
            }
            0
        }
        WM_DESTROYCLIPBOARD => {
            // Another application replaced the clipboard: the queue ends.
            if !RETAKING.load(Ordering::SeqCst) {
                let was_active = std::mem::replace(&mut STATE.lock().active, false);
                if was_active {
                    notify();
                }
            }
            0
        }
        WM_STOP => {
            let text = {
                let mut s = STATE.lock();
                let pending = s.active.then(|| s.items.get(s.position).cloned()).flatten();
                s.active = false;
                pending
            };
            // Replace the announced data by the real item so a later paste works.
            if let Some(text) = text {
                if GetClipboardOwner() == hwnd && open_clipboard(hwnd) {
                    RETAKING.store(true, Ordering::SeqCst);
                    EmptyClipboard();
                    RETAKING.store(false, Ordering::SeqCst);
                    set_text(&text);
                    CloseClipboard();
                }
            }
            notify();
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
