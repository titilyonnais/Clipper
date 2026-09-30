//! Updates from the GitHub releases.
//!
//! Each release publishes `latest.json` next to the installer, signed with
//! Clipper's update key: an installer is only run once its signature matches
//! the public key built into Clipper. It is run silently in update mode,
//! which installs over the current version without running its uninstaller
//! (the start-up entry and Win+V stay as they are) and starts Clipper again.

use crate::commands::{err, CmdResult};
use parking_lot::Mutex;
use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};

/// Written before installing; read at the next start to say it went well.
const NOTICE_FILE: &str = "update-notice";

/// The update found by the last check, ready to install.
#[derive(Default)]
pub struct Pending(Mutex<Option<Update>>);

#[derive(Clone, Serialize)]
pub struct UpdateInfo {
    pub version: String,
    pub current: String,
    pub notes: Option<String>,
    pub date: Option<String>,
}

#[derive(Clone, Serialize)]
struct Progress {
    downloaded: u64,
    total: Option<u64>,
}

fn info(update: &Update) -> UpdateInfo {
    UpdateInfo {
        version: update.version.clone(),
        current: update.current_version.clone(),
        notes: update.body.clone().filter(|b| !b.trim().is_empty()),
        date: update.date.map(|d| d.to_string()),
    }
}

async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    let update = app.updater().map_err(err)?.check().await.map_err(|e| {
        log::info!("update check: {e}");
        "GitHub ne répond pas pour le moment : réessayez plus tard.".to_string()
    })?;
    let found = update.as_ref().map(info);
    *app.state::<Pending>().0.lock() = update;
    Ok(found)
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> CmdResult<Option<UpdateInfo>> {
    check(&app).await
}

/// Download (with progress events), then install and restart. Does not
/// return when the installation starts: Clipper closes for the installer.
#[tauri::command]
pub async fn install_update(app: AppHandle, pending: State<'_, Pending>) -> CmdResult<()> {
    let update = pending
        .0
        .lock()
        .take()
        .ok_or("Aucune mise à jour à installer : vérifiez à nouveau.")?;
    let mut downloaded = 0u64;
    let bytes = update
        .download(
            |chunk, total| {
                downloaded += chunk as u64;
                let _ = app.emit("update:progress", Progress { downloaded, total });
            },
            || {
                let _ = app.emit("update:installing", ());
            },
        )
        .await
        .map_err(|e| format!("Téléchargement interrompu : {e}"))?;
    if let Some(state) = app.try_state::<crate::commands::AppState>() {
        let _ = std::fs::write(
            state.db.data_dir().join(NOTICE_FILE),
            &update.current_version,
        );
    }
    crate::before_exit(&app);
    update
        .install(bytes)
        .map_err(|e| format!("Installation impossible : {e}"))
}

/// After an update installed from Clipper: the version it came from, once.
#[tauri::command]
pub fn take_update_notice(state: State<'_, crate::commands::AppState>) -> Option<String> {
    let path = state.db.data_dir().join(NOTICE_FILE);
    let previous = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    let previous = previous.trim().to_string();
    (!previous.is_empty() && previous != env!("CARGO_PKG_VERSION")).then_some(previous)
}

/// Look for an update shortly after start-up, then every six hours, if the
/// setting allows it; the interface is told when one is available.
pub fn spawn_checks(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio_sleep(Duration::from_secs(20)).await;
        loop {
            let enabled = app
                .try_state::<crate::commands::AppState>()
                .and_then(|s| s.db.get_settings().ok())
                .is_some_and(|s| s.check_updates);
            if enabled {
                match check(&app).await {
                    Ok(Some(found)) => {
                        let _ = app.emit("update:available", found);
                    }
                    Ok(None) => {}
                    Err(_) => {} // Logged by `check`; tried again later.
                }
            }
            tokio_sleep(Duration::from_secs(6 * 3600)).await;
        }
    });
}

async fn tokio_sleep(d: Duration) {
    // The async runtime is Tokio; a plain thread sleep would block a worker.
    let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(d)).await;
}
