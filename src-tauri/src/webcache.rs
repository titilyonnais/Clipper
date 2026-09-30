//! Disk space used by the web view (`%LOCALAPPDATA%\com.clipper.app\EBWebView`).
//!
//! The interface is local, so the browser engine's caches bring little, yet
//! they grow with every version. Both windows start the engine with the
//! arguments of `tauri.conf.json` (small disk cache, no downloaded browser
//! components, sRGB rendering so that nothing flashes when a window appears
//! on an HDR screen), and before any window exists this module removes what is
//! never needed again: crash reports (never sent), downloaded components,
//! and after an update the caches of the previous version.

use std::path::Path;
use tauri::{AppHandle, Manager};

/// Written in the web view folder: the version whose caches it holds.
const VERSION_FILE: &str = "clipper-version";

const ALWAYS: &[&str] = &[
    r"Crashpad\reports",
    r"Crashpad\attachments",
    "component_crx_cache",
    "Subresource Filter",
    "extensions_crx_cache",
];

const AFTER_UPDATE: &[&str] = &[
    r"Default\Cache",
    r"Default\Code Cache",
    r"Default\GPUCache",
    r"Default\DawnGraphiteCache",
    r"Default\DawnWebGPUCache",
    "GrShaderCache",
    "GraphiteDawnCache",
    "ShaderCache",
];

/// Must run before the first window is created.
pub fn tidy(app: &AppHandle) {
    let Ok(dir) = app.path().app_local_data_dir().map(|d| d.join("EBWebView")) else {
        return;
    };
    if !dir.is_dir() {
        return;
    }
    remove_all(&dir, ALWAYS);
    let version = app.package_info().version.to_string();
    let marker = dir.join(VERSION_FILE);
    if std::fs::read_to_string(&marker).ok().as_deref() != Some(version.as_str()) {
        remove_all(&dir, AFTER_UPDATE);
        let _ = std::fs::write(marker, version);
    }
}

fn remove_all(dir: &Path, entries: &[&str]) {
    for entry in entries {
        // Best effort: a file still held by a closing engine stays until next time.
        let _ = std::fs::remove_dir_all(dir.join(entry));
    }
}
