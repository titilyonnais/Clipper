//! Running processes: executable paths and parents.

use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};

pub struct Process {
    pub pid: u32,
    pub parent: u32,
    /// Executable file name, lowercase (`chrome.exe`).
    pub exe: String,
}

/// Every running process.
pub fn snapshot() -> Vec<Process> {
    let mut out = Vec::new();
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return out;
        }
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut ok = Process32FirstW(snap, &mut entry) != 0;
        while ok {
            let len = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            out.push(Process {
                pid: entry.th32ProcessID,
                parent: entry.th32ParentProcessID,
                exe: String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase(),
            });
            ok = Process32NextW(snap, &mut entry) != 0;
        }
        CloseHandle(snap);
    }
    out
}

/// Full path of a process's executable, when it may be queried.
pub fn image_path(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut len) != 0;
        CloseHandle(handle);
        ok.then(|| String::from_utf16_lossy(&buf[..len as usize]))
    }
}

/// WebView2 (`msedgewebview2.exe`) runs in helper processes started by the
/// application that shows it: Clipper itself, Teams, Outlook… Copies made
/// there belong to that application. Returns the host's process id, or
/// `pid` itself for any other process.
pub fn webview_host(pid: u32, exe: &str) -> u32 {
    if !exe.eq_ignore_ascii_case("msedgewebview2.exe") {
        return pid;
    }
    let all = snapshot();
    let mut current = pid;
    // Renderer -> browser process -> host application.
    for _ in 0..4 {
        let Some(p) = all.iter().find(|p| p.pid == current) else {
            break;
        };
        let Some(parent) = all.iter().find(|q| q.pid == p.parent && q.pid != 0) else {
            break;
        };
        current = parent.pid;
        if parent.exe != "msedgewebview2.exe" {
            return current;
        }
    }
    pid
}

/// Processes showing a visible main window: applications people use, as
/// opposed to background services.
pub fn with_windows() -> std::collections::HashSet<u32> {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindow, GetWindowTextLengthW, GetWindowThreadProcessId, IsWindowVisible,
        GW_OWNER,
    };
    unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> i32 {
        let pids = &mut *(data as *mut std::collections::HashSet<u32>);
        if IsWindowVisible(hwnd) != 0
            && GetWindow(hwnd, GW_OWNER).is_null()
            && GetWindowTextLengthW(hwnd) > 0
        {
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            pids.insert(pid);
        }
        1
    }
    let mut pids = std::collections::HashSet::new();
    unsafe { EnumWindows(Some(visit), &mut pids as *mut _ as LPARAM) };
    pids
}
