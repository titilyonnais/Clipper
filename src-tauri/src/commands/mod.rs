//! Commands exposed to the web view. The web view has no file system or
//! process access of its own: everything goes through these functions, which
//! only act on data Clipper stored itself.

pub mod ai;
pub mod clips;
pub mod data;
pub mod organize;
pub mod system;

use crate::clipboard::MonitorState;
use crate::db::Db;
use crate::ocr::Ocr;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

pub struct AppState {
    pub db: Arc<Db>,
    pub monitor: Arc<MonitorState>,
    pub ocr: Ocr,
}

pub type CmdResult<T> = Result<T, String>;

pub fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Tell every window that the history changed.
pub fn changed(app: &AppHandle) {
    let _ = app.emit("clips:changed", ());
}
