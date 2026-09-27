//! Clipboard capture and write-back.
//!
//! On Windows the monitor is event driven (`AddClipboardFormatListener` on a
//! message-only window): nothing runs until another application changes the
//! clipboard, and the clipboard is only opened for the few milliseconds needed
//! to copy the data out.

use crate::db::{Db, NewClip};
use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

/// Texts larger than this are not recorded (keeps the database small).
const MAX_TEXT_BYTES: usize = 8 * 1024 * 1024;
const MAX_IMAGE_BYTES: usize = 40 * 1024 * 1024;
const PREVIEW_CHARS: usize = 280;

#[derive(Default)]
pub struct MonitorState {
    pub paused: AtomicBool,
    /// Clipboard sequence number produced by our own last write, so the
    /// monitor does not record what Clipper itself just copied.
    own_seq: AtomicU32,
}

enum Snapshot {
    Files(Vec<String>),
    Text(String),
    /// PNG-encoded image.
    Image(Vec<u8>),
}

pub fn spawn_monitor(app: AppHandle, db: Arc<Db>, state: Arc<MonitorState>) {
    std::thread::Builder::new()
        .name("clipboard-monitor".into())
        .spawn(move || platform::run(|| on_clipboard_change(&app, &db, &state), &state))
        .expect("failed to start clipboard monitor thread");
}

fn on_clipboard_change(app: &AppHandle, db: &Db, state: &MonitorState) {
    if state.paused.load(Ordering::Relaxed) {
        return;
    }
    if platform::sequence_number() == state.own_seq.load(Ordering::Relaxed) {
        return;
    }
    let settings = db.get_settings().unwrap_or_default();
    let Some((snapshot, source_app)) = platform::capture(&settings.ignore_apps) else {
        log::debug!("clipboard change skipped");
        return;
    };
    log::debug!("clipboard change from {source_app:?}");
    let stored = match snapshot {
        Snapshot::Text(text) => store_text(db, &text, source_app.as_deref()),
        Snapshot::Files(paths) => store_files(db, &paths, source_app.as_deref()),
        Snapshot::Image(png) => store_image(db, &png, source_app.as_deref()),
    };
    match stored {
        Ok(true) => {
            if settings.max_items > 0 {
                if let Err(e) = db.enforce_limit(settings.max_items) {
                    log::warn!("enforce_limit: {e}");
                }
            }
            let _ = app.emit("clips:changed", ());
        }
        Ok(false) => {}
        Err(e) => log::warn!("failed to store clip: {e}"),
    }
}

fn store_text(db: &Db, text: &str, source_app: Option<&str>) -> Result<bool> {
    if text.trim().is_empty() || text.len() > MAX_TEXT_BYTES {
        return Ok(false);
    }
    let (kind, language) = classify(text);
    db.upsert_clip(&NewClip {
        kind,
        content: text,
        preview: &make_preview(text),
        language,
        source_app,
        size_bytes: text.len() as i64,
        hash: &hash_text(text),
    })?;
    Ok(true)
}

fn store_files(db: &Db, paths: &[String], source_app: Option<&str>) -> Result<bool> {
    if paths.is_empty() {
        return Ok(false);
    }
    db.upsert_clip(&NewClip {
        kind: "file",
        content: &serde_json::to_string(paths)?,
        preview: &files_preview(paths),
        language: None,
        source_app,
        size_bytes: paths.iter().map(|p| p.len() as i64).sum(),
        hash: &hash_files(paths),
    })?;
    Ok(true)
}

fn store_image(db: &Db, png: &[u8], source_app: Option<&str>) -> Result<bool> {
    if png.is_empty() || png.len() > MAX_IMAGE_BYTES {
        return Ok(false);
    }
    let (w, h) = png_dimensions(png).ok_or_else(|| anyhow!("invalid PNG"))?;
    let file = db.store_image_file(png)?;
    db.upsert_clip(&NewClip {
        kind: "image",
        content: &file,
        preview: &format!("{w}×{h}"),
        language: None,
        source_app,
        size_bytes: png.len() as i64,
        hash: &hash_image(png),
    })?;
    Ok(true)
}

pub fn hash_text(s: &str) -> String {
    format!("t:{:x}", Sha256::digest(s.as_bytes()))
}

pub fn hash_files(paths: &[String]) -> String {
    format!("f:{:x}", Sha256::digest(paths.join("\n").as_bytes()))
}

pub fn hash_image(png: &[u8]) -> String {
    format!("i:{:x}", Sha256::digest(png))
}

pub fn png_dimensions(png: &[u8]) -> Option<(u32, u32)> {
    // Signature (8) + IHDR length/type (8) + width (4) + height (4).
    if png.len() < 24 || &png[..8] != b"\x89PNG\r\n\x1a\n" || &png[12..16] != b"IHDR" {
        return None;
    }
    let w = u32::from_be_bytes(png[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(png[20..24].try_into().ok()?);
    Some((w, h))
}

pub fn make_preview(text: &str) -> String {
    let collapsed: String = text.trim().chars().take(PREVIEW_CHARS + 1).collect();
    if collapsed.chars().count() > PREVIEW_CHARS {
        let mut s: String = collapsed.chars().take(PREVIEW_CHARS).collect();
        s.push('…');
        s
    } else {
        collapsed
    }
}

pub fn files_preview(paths: &[String]) -> String {
    let names: Vec<&str> = paths
        .iter()
        .take(3)
        .map(|p| {
            std::path::Path::new(p)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(p)
        })
        .collect();
    match paths.len() {
        0 => String::new(),
        1..=3 => names.join(", "),
        n => format!("{} (+{} autres)", names.join(", "), n - 3),
    }
}

/// Returns (kind, language).
pub fn classify(text: &str) -> (&'static str, Option<&'static str>) {
    let t = text.trim();
    let lower_start: String = t.chars().take(12).collect::<String>().to_ascii_lowercase();
    if ["http://", "https://", "ftp://", "mailto:"]
        .iter()
        .any(|p| lower_start.starts_with(p))
        && !t.contains(char::is_whitespace)
    {
        return ("url", None);
    }
    match detect_language(t) {
        Some(lang) => ("code", Some(lang)),
        None => ("text", None),
    }
}

fn detect_language(t: &str) -> Option<&'static str> {
    // Heuristics are cheap but only worth running on text that looks structured.
    let sample: String = t.chars().take(4000).collect();
    let lower = sample.to_ascii_lowercase();
    let lines = sample.lines().count();
    let starts = |p: &str| lower.starts_with(p);
    let has = |p: &str| lower.contains(p);

    let bracketed = (starts("{") && t.ends_with('}')) || (starts("[") && t.ends_with(']'));
    if bracketed && (sample.contains("\":") || serde_json::from_str::<serde_json::Value>(t).is_ok())
    {
        return Some("json");
    }
    if starts("<") && has("</") {
        return Some(if has("<!doctype html") || has("<html") || has("<div") {
            "html"
        } else {
            "xml"
        });
    }
    if starts("#!/") || starts("$ ") || has("sudo ") && lines > 1 {
        return Some("bash");
    }
    let sql_start = [
        "select ",
        "insert into ",
        "update ",
        "delete from ",
        "create table ",
        "with ",
    ];
    if sql_start.iter().any(|p| starts(p))
        && (has(" from ") || has(" set ") || has(" values") || has("("))
    {
        return Some("sql");
    }
    if has("fn ") && (has("let ") || has("->") || has("pub ")) || has("impl ") && has("{") {
        return Some("rust");
    }
    if (has("def ") || has("class ")) && sample.contains(":\n")
        || starts("import ") && !has(";")
        || starts("from ") && has(" import ")
    {
        return Some("python");
    }
    if has("interface ") && has("{") || has(": string") || has(": number") || has("export type ") {
        return Some("typescript");
    }
    if has("function ") && has("{")
        || has("const ") && has(" = ")
        || has("=> {")
        || has("console.log")
    {
        return Some("javascript");
    }
    if has("{") && (has("color:") || has("display:") || has("margin:") || has("padding:")) {
        return Some("css");
    }
    if has("$env:") || has("get-childitem") || has("write-host") {
        return Some("powershell");
    }
    let indented = sample
        .lines()
        .filter(|l| l.starts_with("    ") || l.starts_with('\t'))
        .count();
    let semis = sample.matches(';').count();
    if lines >= 3 && indented >= 2 && (semis >= 3 || (has("{") && has("}"))) {
        return Some("plaintext");
    }
    None
}

// ─── Write-back ───

pub fn write_text(state: &MonitorState, text: &str) -> Result<()> {
    platform::write(Snapshot::Text(text.to_string()))?;
    remember_own_write(state);
    Ok(())
}

pub fn write_files(state: &MonitorState, paths: &[String]) -> Result<()> {
    platform::write(Snapshot::Files(paths.to_vec()))?;
    remember_own_write(state);
    Ok(())
}

pub fn write_png(state: &MonitorState, png: Vec<u8>) -> Result<()> {
    platform::write(Snapshot::Image(png))?;
    remember_own_write(state);
    Ok(())
}

fn remember_own_write(state: &MonitorState) {
    state
        .own_seq
        .store(platform::sequence_number(), Ordering::Relaxed);
}

#[cfg(windows)]
mod platform {
    use super::{MonitorState, Snapshot};
    use anyhow::{anyhow, Result};
    use clipboard_win::{formats, options::NoClear, raw, Clipboard, Getter};
    use std::sync::atomic::Ordering;
    use std::time::Duration;
    use windows_sys::Win32::Foundation::{CloseHandle, HWND};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };

    pub fn sequence_number() -> u32 {
        raw::seq_num().map(|n| n.get()).unwrap_or(0)
    }

    pub fn run(mut on_change: impl FnMut(), state: &MonitorState) {
        let mut monitor = match clipboard_win::Monitor::new() {
            Ok(m) => m,
            Err(e) => {
                log::error!("clipboard listener unavailable: {e}");
                return;
            }
        };
        loop {
            match monitor.recv() {
                Ok(true) => {}
                Ok(false) => return,
                Err(e) => {
                    log::warn!("clipboard listener error: {e}");
                    std::thread::sleep(Duration::from_secs(1));
                    continue;
                }
            }
            // Applications often update the clipboard several times in a row
            // (one call per format). Wait briefly and handle only the last one.
            std::thread::sleep(Duration::from_millis(80));
            while matches!(monitor.try_recv(), Ok(true)) {}
            if !state.paused.load(Ordering::Relaxed) {
                on_change();
            }
        }
    }

    fn registered(name: &str) -> Option<u32> {
        clipboard_win::register_format(name).map(|n| n.get())
    }

    /// Password managers and other sensitive apps flag their clipboard data
    /// with these formats. Clipper honours them like Windows' own history.
    fn excluded_by_owner() -> bool {
        for name in [
            "ExcludeClipboardContentFromMonitorProcessing",
            "Clipboard Viewer Ignore",
        ] {
            if registered(name).is_some_and(raw::is_format_avail) {
                return true;
            }
        }
        if let Some(fmt) =
            registered("CanIncludeInClipboardHistory").filter(|f| raw::is_format_avail(*f))
        {
            let mut data = Vec::new();
            if formats::RawData(fmt).read_clipboard(&mut data).is_ok() && data.len() >= 4 {
                return u32::from_le_bytes([data[0], data[1], data[2], data[3]]) == 0;
            }
        }
        false
    }

    /// Executable name of the application that owns the clipboard. Programs
    /// that open the clipboard without a window have no owner; the foreground
    /// application is the best guess then (unless it is Clipper itself).
    fn owner_app() -> Option<String> {
        let owner = raw::get_owner().map(|p| p.as_ptr() as HWND);
        let hwnd = owner.unwrap_or_else(|| unsafe { GetForegroundWindow() });
        if hwnd.is_null() {
            return None;
        }
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
        if pid == 0 {
            return None;
        }
        if pid == std::process::id() {
            return owner.map(|_| String::new());
        }
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                return None;
            }
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut len) != 0;
            CloseHandle(handle);
            if !ok {
                return None;
            }
            let path = String::from_utf16_lossy(&buf[..len as usize]);
            std::path::Path::new(&path)
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
        }
    }

    pub fn capture(ignore_apps: &[String]) -> Option<(Snapshot, Option<String>)> {
        let app = owner_app();
        if let Some(name) = &app {
            if name.is_empty() {
                return None; // Clipper's own write.
            }
            let ignored = ignore_apps.iter().any(|a| {
                let a = a.trim().to_lowercase();
                !a.is_empty()
                    && (*name == a || name.trim_end_matches(".exe") == a.trim_end_matches(".exe"))
            });
            if ignored {
                return None;
            }
        }

        let _guard = Clipboard::new_attempts(10).ok()?;
        if excluded_by_owner() {
            return None;
        }
        let snapshot = read_snapshot()?;
        Some((snapshot, app))
    }

    /// Reads the most useful format. Must be called with the clipboard open.
    fn read_snapshot() -> Option<Snapshot> {
        if raw::is_format_avail(formats::CF_HDROP) {
            let mut files: Vec<String> = Vec::new();
            if formats::FileList.read_clipboard(&mut files).is_ok() && !files.is_empty() {
                return Some(Snapshot::Files(files));
            }
        }
        if raw::is_format_avail(formats::CF_UNICODETEXT) {
            let mut text = String::new();
            if formats::Unicode.read_clipboard(&mut text).is_ok() && !text.trim().is_empty() {
                return Some(Snapshot::Text(text));
            }
        }
        if let Some(png_fmt) = registered("PNG").filter(|f| raw::is_format_avail(*f)) {
            let mut png = Vec::new();
            if formats::RawData(png_fmt).read_clipboard(&mut png).is_ok()
                && super::png_dimensions(&png).is_some()
            {
                return Some(Snapshot::Image(png));
            }
        }
        if raw::is_format_avail(formats::CF_BITMAP) {
            let mut bmp = Vec::new();
            if formats::Bitmap.read_clipboard(&mut bmp).is_ok() {
                return bmp_to_png(&bmp).map(Snapshot::Image);
            }
        }
        None
    }

    fn bmp_to_png(bmp: &[u8]) -> Option<Vec<u8>> {
        let img = image::load_from_memory_with_format(bmp, image::ImageFormat::Bmp).ok()?;
        // CF_BITMAP carries no usable alpha channel.
        let rgb = img.to_rgb8();
        let mut png = Vec::new();
        rgb.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .ok()?;
        Some(png)
    }

    /// 32-bit bottom-up BMP (BITMAPFILEHEADER + BITMAPINFOHEADER) from a PNG.
    fn png_to_bmp(png: &[u8]) -> Result<Vec<u8>> {
        let img = image::load_from_memory_with_format(png, image::ImageFormat::Png)?.to_rgba8();
        let (w, h) = img.dimensions();
        let pixels = (w as usize) * (h as usize) * 4;
        let mut out = Vec::with_capacity(54 + pixels);
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&(54 + pixels as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&54u32.to_le_bytes());
        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&(w as i32).to_le_bytes());
        out.extend_from_slice(&(h as i32).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
        out.extend_from_slice(&(pixels as u32).to_le_bytes());
        out.extend_from_slice(&[0u8; 16]);
        for row in img.rows().rev() {
            for p in row {
                out.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
            }
        }
        Ok(out)
    }

    pub fn write(snapshot: Snapshot) -> Result<()> {
        // Heavy conversion happens before the clipboard is opened.
        let bmp = match &snapshot {
            Snapshot::Image(png) => Some(png_to_bmp(png)?),
            _ => None,
        };
        let _guard =
            Clipboard::new_attempts(10).map_err(|e| anyhow!("presse-papiers occupé : {e}"))?;
        raw::empty().map_err(|e| anyhow!("{e}"))?;
        let res = match &snapshot {
            Snapshot::Text(text) => raw::set_string_with(text, NoClear),
            Snapshot::Files(paths) => raw::set_file_list_with(paths, NoClear),
            Snapshot::Image(png) => {
                if let Some(fmt) = registered("PNG") {
                    let _ = raw::set_without_clear(fmt, png);
                }
                raw::set_bitmap_with(bmp.as_deref().unwrap_or_default(), NoClear)
            }
        };
        res.map_err(|e| anyhow!("écriture dans le presse-papiers impossible : {e}"))
    }
}

#[cfg(not(windows))]
mod platform {
    use super::{MonitorState, Snapshot};
    use anyhow::{anyhow, Result};

    pub fn sequence_number() -> u32 {
        0
    }

    pub fn run(_on_change: impl FnMut(), _state: &MonitorState) {
        log::warn!("clipboard monitoring is only implemented on Windows");
    }

    pub fn capture(_ignore_apps: &[String]) -> Option<(Snapshot, Option<String>)> {
        None
    }

    pub fn write(_snapshot: Snapshot) -> Result<()> {
        Err(anyhow!("clipboard writing is only implemented on Windows"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_content() {
        assert_eq!(classify("https://example.com/a?b=c"), ("url", None));
        assert_eq!(classify("see https://example.com"), ("text", None));
        assert_eq!(classify("{\"a\": 1}"), ("code", Some("json")));
        assert_eq!(
            classify("SELECT id FROM users WHERE x = 1"),
            ("code", Some("sql"))
        );
        assert_eq!(classify("Bonjour, à demain !"), ("text", None));
        assert_eq!(
            classify("Quand je suis sur l'interface d'un client : const et class"),
            ("text", None)
        );
        assert_eq!(
            classify("fn main() {\n    let x = 1;\n}"),
            ("code", Some("rust"))
        );
    }

    #[test]
    fn preview_is_char_safe() {
        let s = "é".repeat(400);
        let p = make_preview(&s);
        assert_eq!(p.chars().count(), PREVIEW_CHARS + 1);
        assert!(p.ends_with('…'));
    }

    #[test]
    fn files_preview_summarizes() {
        let paths: Vec<String> = (1..=5).map(|i| format!("C:\\dir\\f{i}.txt")).collect();
        assert_eq!(files_preview(&paths), "f1.txt, f2.txt, f3.txt (+2 autres)");
    }
}
