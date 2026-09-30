//! Clipboard capture and write-back.
//!
//! The monitor is event driven (`AddClipboardFormatListener` on a message-only
//! window): nothing runs until another application changes the clipboard, and
//! the clipboard is only opened for the few milliseconds needed to copy the
//! data out.

mod classify;
pub mod queue;
mod win;

pub use classify::{
    classify, files_preview, hash_files, hash_image, hash_text, make_preview, png_dimensions,
    sha256_hex,
};
pub use win::{process_of_window, read_text, Payload};

use crate::db::{Db, NewClip, RichFormats};
use anyhow::{anyhow, Result};
use std::sync::atomic::{AtomicI64, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Capture state shared by the monitor, the commands and the tray.
#[derive(Default)]
pub struct MonitorState {
    /// 0 = capturing, `i64::MAX` = paused until resumed, otherwise the Unix
    /// time (seconds) at which capture resumes.
    paused_until: AtomicI64,
    /// Clipboard sequence number produced by our own last write, so the
    /// monitor does not record what Clipper itself just copied.
    own_seq: AtomicU32,
}

impl MonitorState {
    pub fn is_paused(&self) -> bool {
        let until = self.paused_until.load(Ordering::Relaxed);
        until != 0 && (until == i64::MAX || chrono::Utc::now().timestamp() < until)
    }

    /// `None` pauses until resumed, `Some(d)` for a duration.
    pub fn pause(&self, duration: Option<Duration>) {
        let until = match duration {
            None => i64::MAX,
            Some(d) => chrono::Utc::now().timestamp() + d.as_secs() as i64,
        };
        self.paused_until.store(until, Ordering::Relaxed);
    }

    pub fn resume(&self) {
        self.paused_until.store(0, Ordering::Relaxed);
    }

    /// RFC 3339 end of a timed pause, "forever", or `None` when capturing.
    pub fn paused_until(&self) -> Option<String> {
        if !self.is_paused() {
            return None;
        }
        match self.paused_until.load(Ordering::Relaxed) {
            i64::MAX => Some("forever".into()),
            t => chrono::DateTime::from_timestamp(t, 0).map(|d| d.to_rfc3339()),
        }
    }
}

/// What the monitor just stored, for follow-up work (OCR, icons, UI refresh).
pub struct Stored {
    pub id: i64,
    pub kind: &'static str,
    pub app_path: Option<String>,
}

pub fn spawn_monitor(
    db: Arc<Db>,
    state: Arc<MonitorState>,
    on_stored: impl Fn(Stored) + Send + 'static,
) {
    std::thread::Builder::new()
        .name("clipboard-monitor".into())
        .spawn(move || run(|| on_clipboard_change(&db, &state, &on_stored)))
        .expect("failed to start clipboard monitor thread");
}

fn run(mut on_change: impl FnMut()) {
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
        on_change();
    }
}

fn on_clipboard_change(db: &Db, state: &MonitorState, on_stored: &impl Fn(Stored)) {
    if state.is_paused() || win::sequence_number() == state.own_seq.load(Ordering::Relaxed) {
        return;
    }
    let settings = db.get_settings().unwrap_or_default();
    let captured = match win::capture(&settings.ignore_apps, settings.keep_rich_text) {
        Ok(c) => c,
        Err(reason) => {
            log::debug!("not recorded: {reason:?}");
            return;
        }
    };
    let app_name = captured.app.as_ref().map(|a| a.name.as_str());
    let stored = match &captured.content {
        win::Content::Text(text) => {
            store_text(db, text, &captured.rich, app_name, settings.detect_secrets)
        }
        win::Content::Files(paths) => store_files(db, paths, app_name),
        win::Content::Image(png) => store_image(db, png, app_name),
    };
    match stored {
        Ok(Some((id, kind))) => {
            if settings.max_items > 0 {
                if let Err(e) = db.enforce_limit(settings.max_items) {
                    log::warn!("enforce_limit: {e}");
                }
            }
            on_stored(Stored {
                id,
                kind,
                app_path: captured.app.map(|a| a.path),
            });
        }
        Ok(None) => {}
        Err(e) => log::warn!("failed to store clip: {e}"),
    }
}

type StoreResult = Result<Option<(i64, &'static str)>>;

fn store_text(
    db: &Db,
    text: &str,
    rich: &RichFormats,
    source_app: Option<&str>,
    detect_secrets: bool,
) -> StoreResult {
    if text.trim().is_empty() || text.len() > win::MAX_TEXT_BYTES {
        return Ok(None);
    }
    let (kind, language) = classify(text);
    let sensitive = detect_secrets && crate::sensitive::detect(text).is_some();
    // Never keep formatted copies of a secret.
    let no_rich = RichFormats::default();
    let id = db.upsert_clip(&NewClip {
        kind,
        content: text,
        preview: &make_preview(text),
        language,
        source_app,
        size_bytes: text.len() as i64,
        hash: &hash_text(text),
        sensitive,
        rich: if sensitive { &no_rich } else { rich },
    })?;
    Ok(Some((id, kind)))
}

fn store_files(db: &Db, paths: &[String], source_app: Option<&str>) -> StoreResult {
    if paths.is_empty() {
        return Ok(None);
    }
    let id = db.upsert_clip(&NewClip {
        kind: "file",
        content: &serde_json::to_string(paths)?,
        preview: &files_preview(paths),
        language: None,
        source_app,
        size_bytes: paths.iter().map(|p| p.len() as i64).sum(),
        hash: &hash_files(paths),
        sensitive: false,
        rich: &RichFormats::default(),
    })?;
    Ok(Some((id, "file")))
}

fn store_image(db: &Db, png: &[u8], source_app: Option<&str>) -> StoreResult {
    if png.is_empty() || png.len() > win::MAX_IMAGE_BYTES {
        return Ok(None);
    }
    let (w, h) = png_dimensions(png).ok_or_else(|| anyhow!("invalid PNG"))?;
    let file = db.store_image_file(png)?;
    let id = db.upsert_clip(&NewClip {
        kind: "image",
        content: &file,
        preview: &format!("{w}×{h}"),
        language: None,
        source_app,
        size_bytes: png.len() as i64,
        hash: &hash_image(png),
        sensitive: false,
        rich: &RichFormats::default(),
    })?;
    Ok(Some((id, "image")))
}

/// Put data on the clipboard without recording it as a new clip.
pub fn write(state: &MonitorState, payload: Payload) -> Result<()> {
    win::write(payload)?;
    state
        .own_seq
        .store(win::sequence_number(), Ordering::Relaxed);
    Ok(())
}
