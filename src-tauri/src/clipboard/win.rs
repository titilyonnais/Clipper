//! Direct access to the Windows clipboard.

use crate::db::RichFormats;
use anyhow::{anyhow, Result};
use clipboard_win::{formats, options::NoClear, raw, Clipboard, Getter};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

/// Texts larger than this are not recorded (keeps the database small).
pub const MAX_TEXT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_IMAGE_BYTES: usize = 40 * 1024 * 1024;
const MAX_RICH_BYTES: usize = 4 * 1024 * 1024;

pub enum Content {
    Files(Vec<String>),
    Text(String),
    /// PNG-encoded image.
    Image(Vec<u8>),
}

pub struct Captured {
    pub content: Content,
    pub rich: RichFormats,
    pub app: Option<OwnerApp>,
}

pub struct OwnerApp {
    /// Lower-case executable name, e.g. "chrome.exe".
    pub name: String,
    pub path: String,
}

/// Why a clipboard change is not recorded (logged at debug level).
#[derive(Debug)]
pub enum Skip {
    OwnWrite,
    Ignored,
    Private,
    Empty,
}

pub fn sequence_number() -> u32 {
    raw::seq_num().map(|n| n.get()).unwrap_or(0)
}

pub fn registered(name: &str) -> Option<u32> {
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

pub fn process_of_window(hwnd: HWND) -> Option<(u32, String)> {
    if hwnd.is_null() {
        return None;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    (pid != 0).then(|| (pid, crate::processes::image_path(pid).unwrap_or_default()))
}

/// Application that owns the clipboard. Programs that open the clipboard
/// without a window have no owner; the foreground application is the best
/// guess then (unless it is Clipper itself).
fn owner_app() -> Result<Option<OwnerApp>, Skip> {
    let owner = raw::get_owner().map(|p| p.as_ptr() as HWND);
    let hwnd = owner.unwrap_or_else(|| unsafe { GetForegroundWindow() });
    let Some((pid, path)) = process_of_window(hwnd) else {
        return Ok(None);
    };
    if pid == std::process::id() {
        return if owner.is_some() {
            Err(Skip::OwnWrite)
        } else {
            Ok(None)
        };
    }
    let file_name = |path: &str| {
        std::path::Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    };
    // A copy made in a web view belongs to the application showing it
    // (Clipper's own interface included).
    let host = crate::processes::webview_host(pid, &file_name(&path));
    let path = if host == pid {
        path
    } else if host == std::process::id() {
        std::env::current_exe()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
    } else {
        crate::processes::image_path(host).unwrap_or(path)
    };
    let name = file_name(&path);
    Ok((!name.is_empty()).then_some(OwnerApp { name, path }))
}

pub fn is_ignored(name: &str, ignore_apps: &[String]) -> bool {
    ignore_apps.iter().any(|a| {
        let a = a.trim().to_lowercase();
        !a.is_empty() && name.trim_end_matches(".exe") == a.trim_end_matches(".exe")
    })
}

pub fn capture(ignore_apps: &[String], keep_rich: bool) -> Result<Captured, Skip> {
    let app = owner_app()?;
    if app
        .as_ref()
        .is_some_and(|a| is_ignored(&a.name, ignore_apps))
    {
        return Err(Skip::Ignored);
    }
    // Other applications cannot use the clipboard while it is open: copy the
    // raw data, close it, and only then convert anything.
    let (read, rich) = {
        let _guard = Clipboard::new_attempts(10).map_err(|_| Skip::Empty)?;
        if excluded_by_owner() {
            return Err(Skip::Private);
        }
        let read = read_content().ok_or(Skip::Empty)?;
        let rich = match &read {
            Read::Ready(Content::Text(_)) if keep_rich => read_rich(),
            _ => RichFormats::default(),
        };
        (read, rich)
    };
    let content = match read {
        Read::Ready(content) => content,
        Read::Bitmap(bmp) => Content::Image(bmp_to_png(&bmp).ok_or(Skip::Empty)?),
    };
    Ok(Captured { content, rich, app })
}

/// What was read while the clipboard was open.
enum Read {
    Ready(Content),
    /// A device-independent bitmap, converted to PNG once the clipboard is closed.
    Bitmap(Vec<u8>),
}

fn read_raw(format: u32, max: usize) -> Option<Vec<u8>> {
    if !raw::is_format_avail(format) || raw::size(format).is_some_and(|s| s.get() > max) {
        return None;
    }
    let mut data = Vec::new();
    formats::RawData(format)
        .read_clipboard(&mut data)
        .ok()
        .filter(|_| !data.is_empty())
        .map(|_| data)
}

fn read_rich() -> RichFormats {
    RichFormats {
        html: registered("HTML Format").and_then(|f| read_raw(f, MAX_RICH_BYTES)),
        rtf: registered("Rich Text Format").and_then(|f| read_raw(f, MAX_RICH_BYTES)),
    }
}

/// Reads the most useful format. Must be called with the clipboard open.
fn read_content() -> Option<Read> {
    if raw::is_format_avail(formats::CF_HDROP) {
        let mut files: Vec<String> = Vec::new();
        if formats::FileList.read_clipboard(&mut files).is_ok() && !files.is_empty() {
            return Some(Read::Ready(Content::Files(files)));
        }
    }
    if raw::is_format_avail(formats::CF_UNICODETEXT)
        && raw::size(formats::CF_UNICODETEXT).is_none_or(|s| s.get() <= MAX_TEXT_BYTES * 2)
    {
        let mut text = String::new();
        if formats::Unicode.read_clipboard(&mut text).is_ok() && !text.trim().is_empty() {
            return Some(Read::Ready(Content::Text(text)));
        }
    }
    if let Some(png) = registered("PNG").and_then(|f| read_raw(f, MAX_IMAGE_BYTES)) {
        if super::png_dimensions(&png).is_some() {
            return Some(Read::Ready(Content::Image(png)));
        }
    }
    if raw::is_format_avail(formats::CF_BITMAP) {
        let mut bmp = Vec::new();
        if formats::Bitmap.read_clipboard(&mut bmp).is_ok() {
            return Some(Read::Bitmap(bmp));
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

/// What to put on the clipboard.
pub enum Payload<'a> {
    Text {
        text: &'a str,
        rich: Option<&'a RichFormats>,
    },
    Files(&'a [String]),
    Png(&'a [u8]),
}

pub fn write(payload: Payload) -> Result<()> {
    // Heavy conversion happens before the clipboard is opened.
    let bmp = match &payload {
        Payload::Png(png) => Some(png_to_bmp(png)?),
        _ => None,
    };
    let _guard = Clipboard::new_attempts(20)
        .map_err(|_| anyhow!("Le presse-papiers est occupé par une autre application."))?;
    raw::empty().map_err(|e| anyhow!("{e}"))?;
    let res = match &payload {
        Payload::Text { text, rich } => {
            if let Some(rich) = rich {
                for (name, data) in [("HTML Format", &rich.html), ("Rich Text Format", &rich.rtf)] {
                    if let (Some(fmt), Some(data)) = (registered(name), data) {
                        let _ = raw::set_without_clear(fmt, data);
                    }
                }
            }
            raw::set_string_with(text, NoClear)
        }
        Payload::Files(paths) => raw::set_file_list_with(paths, NoClear),
        Payload::Png(png) => {
            if let Some(fmt) = registered("PNG") {
                let _ = raw::set_without_clear(fmt, png);
            }
            raw::set_bitmap_with(bmp.as_deref().unwrap_or_default(), NoClear)
        }
    };
    res.map_err(|e| anyhow!("Écriture dans le presse-papiers impossible : {e}"))
}

/// Current clipboard text, if any (for the `{presse-papiers}` snippet variable).
pub fn read_text() -> Option<String> {
    let _guard = Clipboard::new_attempts(10).ok()?;
    let mut text = String::new();
    formats::Unicode.read_clipboard(&mut text).ok()?;
    Some(text)
}
