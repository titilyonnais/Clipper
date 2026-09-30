//! Icons of the applications clips come from, cached as PNG files named
//! after the executable (e.g. `icons/chrome.exe.png`).
//!
//! An icon is extracted when a clip arrives from a new application. Those
//! that could not be (application closed, no rights at that moment) are
//! looked for again during maintenance, by finding the executable among
//! running processes, the registered applications and the `PATH`.

use std::path::{Path, PathBuf};
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
};
use windows_sys::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, GetIconInfo, PrivateExtractIconsW, HICON, ICONINFO,
};

/// Side of the extracted icons: sharp at 200 % where they are shown at 16 to 20 px.
const SIDE: i32 = 64;

pub fn icon_path(dir: &Path, exe_name: &str) -> PathBuf {
    let safe: String = exe_name
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .collect();
    dir.join(format!("{safe}.png"))
}

/// Extract and cache the icon of `exe_path` if it is not cached yet.
/// Returns whether a new icon was written.
pub fn ensure_icon(dir: &Path, exe_path: &str) -> bool {
    let Some(name) = Path::new(exe_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
    else {
        return false;
    };
    let dest = icon_path(dir, &name);
    if dest.exists() {
        return false;
    }
    let Some(png) = extract_png(exe_path) else {
        return false;
    };
    let _ = std::fs::create_dir_all(dir);
    std::fs::write(dest, png).is_ok()
}

/// Look for the icons still missing among `apps` (executable names).
/// Returns the number of icons written.
pub fn backfill(dir: &Path, apps: &[String]) -> usize {
    let missing: Vec<&String> = apps
        .iter()
        .filter(|a| !icon_path(dir, a).exists())
        .collect();
    if missing.is_empty() {
        return 0;
    }
    let running = crate::processes::snapshot();
    missing
        .into_iter()
        .filter_map(|app| find_exe(app, &running))
        .filter(|path| ensure_icon(dir, path))
        .count()
}

/// Where the executable `name` is: a running copy, the applications
/// registered with Windows, or the `PATH`.
fn find_exe(name: &str, running: &[crate::processes::Process]) -> Option<String> {
    let name = name.to_lowercase();
    if !name.ends_with(".exe") || name.contains(['\\', '/']) {
        return None;
    }
    if let Some(path) = running
        .iter()
        .filter(|p| p.exe == name)
        .find_map(|p| crate::processes::image_path(p.pid))
    {
        return Some(path);
    }
    let registered = crate::registry::read_any_hive(
        &format!(r"Software\Microsoft\Windows\CurrentVersion\App Paths\{name}"),
        "",
    )
    .map(|p| p.trim_matches('"').to_string())
    .filter(|p| Path::new(p).is_file());
    registered.or_else(|| {
        std::env::var_os("PATH").and_then(|paths| {
            std::env::split_paths(&paths)
                .map(|d| d.join(&name))
                .find(|p| p.is_file())
                .map(|p| p.to_string_lossy().into_owned())
        })
    })
}

fn extract_png(exe_path: &str) -> Option<Vec<u8>> {
    let wide: Vec<u16> = exe_path.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        // The large version from the file itself, else the shell's 32 px one.
        let mut icon: HICON = std::ptr::null_mut();
        let mut id = 0u32;
        let found = PrivateExtractIconsW(wide.as_ptr(), 0, SIDE, SIDE, &mut icon, &mut id, 1, 0);
        if found == 1 && !icon.is_null() {
            let png = icon_to_png(icon);
            DestroyIcon(icon);
            if png.is_some() {
                return png;
            }
        }
        let mut info: SHFILEINFOW = std::mem::zeroed();
        let ok = SHGetFileInfoW(
            wide.as_ptr(),
            0,
            &mut info,
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        );
        if ok == 0 || info.hIcon.is_null() {
            return None;
        }
        let png = icon_to_png(info.hIcon);
        DestroyIcon(info.hIcon);
        png
    }
}

unsafe fn icon_to_png(icon: windows_sys::Win32::UI::WindowsAndMessaging::HICON) -> Option<Vec<u8>> {
    let mut ii: ICONINFO = std::mem::zeroed();
    if GetIconInfo(icon, &mut ii) == 0 {
        return None;
    }
    let result = (|| {
        if ii.hbmColor.is_null() {
            return None;
        }
        let mut bm: BITMAP = std::mem::zeroed();
        if GetObjectW(
            ii.hbmColor,
            std::mem::size_of::<BITMAP>() as i32,
            &mut bm as *mut _ as *mut _,
        ) == 0
        {
            return None;
        }
        let (w, h) = (bm.bmWidth, bm.bmHeight);
        if w <= 0 || h <= 0 || w > 256 || h > 256 {
            return None;
        }
        let mut header: BITMAPINFO = std::mem::zeroed();
        header.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..std::mem::zeroed()
        };
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        let dc = CreateCompatibleDC(std::ptr::null_mut());
        let lines = GetDIBits(
            dc,
            ii.hbmColor,
            0,
            h as u32,
            pixels.as_mut_ptr() as *mut _,
            &mut header,
            DIB_RGB_COLORS,
        );
        DeleteDC(dc);
        if lines == 0 {
            return None;
        }
        // BGRA -> RGBA. Old icons without an alpha channel are fully opaque.
        let has_alpha = pixels.chunks_exact(4).any(|p| p[3] != 0);
        for p in pixels.chunks_exact_mut(4) {
            p.swap(0, 2);
            if !has_alpha {
                p[3] = 255;
            }
        }
        let img = image::RgbaImage::from_raw(w as u32, h as u32, pixels)?;
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .ok()?;
        Some(png)
    })();
    if !ii.hbmColor.is_null() {
        DeleteObject(ii.hbmColor);
    }
    if !ii.hbmMask.is_null() {
        DeleteObject(ii.hbmMask);
    }
    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn extracts_a_large_icon() {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
        let png = super::extract_png(&format!(r"{windir}\explorer.exe")).expect("icon");
        let img = image::load_from_memory(&png).unwrap();
        assert_eq!(img.width(), super::SIDE as u32);
    }

    #[test]
    fn finds_executables_on_the_path() {
        let path = super::find_exe("cmd.exe", &[]).expect("cmd.exe");
        assert!(path.to_lowercase().ends_with(r"\cmd.exe"));
        assert!(super::find_exe(r"..\cmd.exe", &[]).is_none());
    }
}
