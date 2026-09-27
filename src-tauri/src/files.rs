use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct FileInfo {
    pub path: String,
    pub exists: bool,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<String>,
    pub is_image: bool,
}

const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "avif"];

/// Extensions that run code when opened. Clipper only reveals them in Explorer.
const EXECUTABLE_EXTS: &[&str] = &[
    "exe",
    "com",
    "bat",
    "cmd",
    "scr",
    "pif",
    "msi",
    "msp",
    "ps1",
    "psm1",
    "vbs",
    "vbe",
    "js",
    "jse",
    "wsf",
    "wsh",
    "hta",
    "cpl",
    "jar",
    "lnk",
    "reg",
    "url",
    "appref-ms",
    "msc",
    "scf",
];

pub const MAX_PREVIEW_BYTES: u64 = 15 * 1024 * 1024;

fn ext(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

pub fn is_executable(path: &str) -> bool {
    EXECUTABLE_EXTS.contains(&ext(path).as_str())
}

pub fn info(path: &str) -> FileInfo {
    let meta = std::fs::metadata(path).ok();
    let is_dir = meta.as_ref().is_some_and(|m| m.is_dir());
    FileInfo {
        path: path.to_string(),
        exists: meta.is_some(),
        is_dir,
        size: meta.as_ref().map_or(0, |m| m.len()),
        modified: meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()),
        is_image: !is_dir && IMAGE_EXTS.contains(&ext(path).as_str()),
    }
}
