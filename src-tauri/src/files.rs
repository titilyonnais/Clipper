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

/// Extensions that run code, install something, mount a disk or reach the
/// network when opened. Clipper only reveals these files in Explorer.
const EXECUTABLE_EXTS: &[&str] = &[
    "exe",
    "com",
    "bat",
    "cmd",
    "scr",
    "pif",
    "msi",
    "msp",
    "mst",
    "ps1",
    "psm1",
    "psd1",
    "ps1xml",
    "ps2",
    "psc1",
    "vbs",
    "vbe",
    "vb",
    "js",
    "jse",
    "wsf",
    "wsh",
    "ws",
    "wsc",
    "sct",
    "hta",
    "cpl",
    "jar",
    "jnlp",
    "lnk",
    "reg",
    "url",
    "website",
    "appref-ms",
    "application",
    "xbap",
    "msc",
    "scf",
    "chm",
    "hlp",
    "xll",
    "inf",
    "gadget",
    "diagcab",
    "settingcontent-ms",
    "library-ms",
    "search-ms",
    "searchconnector-ms",
    "appinstaller",
    "appx",
    "appxbundle",
    "msix",
    "msixbundle",
    "iso",
    "img",
    "vhd",
    "vhdx",
    "py",
    "pyw",
    "pyc",
    "rdp",
    "theme",
    "themepack",
    "deskthemepack",
];

pub const MAX_PREVIEW_BYTES: u64 = 15 * 1024 * 1024;

fn ext(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// Whether opening the file could run code. The real path is resolved
/// first: Windows ignores trailing dots and spaces ("x.exe." opens x.exe)
/// and short 8.3 names hide the extension. Unresolvable paths count as unsafe.
pub fn is_executable(path: &str) -> bool {
    let Ok(real) = std::fs::canonicalize(path) else {
        return true;
    };
    let risky = |p: &str| EXECUTABLE_EXTS.contains(&ext(p).as_str());
    risky(&real.to_string_lossy()) || risky(path.trim_end_matches(['.', ' ']))
}

/// A plain local path (`C:\dossier\fichier`), not a network share or a
/// device path. Imported file lists are restricted to those: displaying a
/// network path makes Windows contact that server.
pub fn is_local_path(path: &str) -> bool {
    let b = path.as_bytes();
    b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/')
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executables_are_recognised_through_windows_path_quirks() {
        let dir = std::env::temp_dir().join(format!("clipper-files-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("outil.exe");
        let doc = dir.join("note.txt");
        std::fs::write(&exe, b"x").unwrap();
        std::fs::write(&doc, b"x").unwrap();
        let exe = exe.to_string_lossy().into_owned();
        assert!(is_executable(&exe));
        assert!(is_executable(&format!("{exe}.")), "trailing dot");
        assert!(is_executable(&format!("{exe} ")), "trailing space");
        assert!(!is_executable(&doc.to_string_lossy()));
        assert!(
            is_executable(&dir.join("absent.txt").to_string_lossy()),
            "unresolvable"
        );
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn only_local_paths_are_accepted_from_imports() {
        assert!(is_local_path(r"C:\Users\moi\a.pdf"));
        assert!(!is_local_path(r"\serveur\partage\a.pdf"));
        assert!(!is_local_path(r"\?\C:\a.pdf"));
        assert!(!is_local_path("a.pdf"));
    }
}
