//! Start with Windows, through the per-user `Run` value that Task Manager
//! lists under "Startup apps".
//!
//! Installing a new version over an old one runs the old uninstaller, which
//! deletes the `Run` value. Clipper therefore checks the entry at every
//! start and writes it again when it should exist, unless the user turned
//! Clipper off in Task Manager (the `StartupApproved` flag), which wins.

use crate::registry::Key;
use tauri::AppHandle;

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
const NAME: &str = "Clipper";
/// `StartupApproved` data for an enabled entry (disabled entries have an odd
/// first byte followed by the time they were disabled).
const APPROVED_ON: [u8; 12] = [2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

/// Only the installed application touches the registry: development and
/// test builds would otherwise point the entry at themselves.
pub fn managed(app: &AppHandle) -> bool {
    !cfg!(debug_assertions) && app.config().identifier == "com.clipper.app"
}

/// Quoted path of this executable, started hidden in the notification area.
fn command() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(format!("\"{}\" --minimized", exe.display()))
}

/// What Task Manager says: `Some(false)` when the user turned Clipper off.
fn approved() -> Option<bool> {
    let data = Key::open(APPROVED, false).ok()?.binary(NAME)?;
    data.first().map(|b| b & 1 == 0)
}

pub fn enable() -> Result<(), String> {
    Key::open(RUN, true)?.set_string(NAME, &command()?)?;
    // Turning it on in Clipper also lifts a Task Manager "Disabled".
    Key::open(APPROVED, true)?.set_binary(NAME, &APPROVED_ON)
}

pub fn disable() -> Result<(), String> {
    Key::open(RUN, true)?.delete(NAME)?;
    Key::open(APPROVED, true)?.delete(NAME)
}

/// Bring the registry in line with the setting at start-up and return
/// whether Clipper will actually start with Windows.
///
/// An enabled `StartupApproved` entry without its `Run` value means an
/// update removed the entry of a user who had turned the option on.
pub fn sync(wanted: bool) -> bool {
    let approved = approved();
    if approved == Some(false) {
        return false;
    }
    let current = Key::open(RUN, false).ok().and_then(|k| k.string(NAME));
    if wanted || approved == Some(true) {
        let up_to_date = matches!((&current, command()), (Some(c), Ok(cmd)) if *c == cmd);
        if up_to_date {
            return true;
        }
        return match enable() {
            Ok(()) => true,
            Err(e) => {
                log::warn!("autostart: {e}");
                false
            }
        };
    }
    current.is_some()
}
