use super::{changed, err, AppState, CmdResult};
use crate::models::{BackupInfo, Settings, SettingsView};
use crate::{backup, credentials, hotkey, tray, window};
use std::time::Duration;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt as _;

/// Both windows keep a copy of the settings: tell them to reload it.
fn settings_changed(app: &AppHandle) {
    let _ = tauri::Emitter::emit(app, "settings:changed", ());
}

pub fn settings_view(state: &AppState, settings: Settings) -> SettingsView {
    SettingsView {
        win_v_active: settings.shortcut_mode == "win_v" && crate::shortcut_registered(),
        settings,
        openai_key_set: credentials::is_set("openai"),
        anthropic_key_set: credentials::is_set("anthropic"),
        data_dir: state.db.data_dir().to_string_lossy().into_owned(),
        paused_until: state.monitor.paused_until(),
    }
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> CmdResult<SettingsView> {
    let settings = state.db.get_settings().map_err(err)?;
    Ok(settings_view(&state, settings))
}

#[tauri::command]
pub async fn set_settings(
    mut settings: Settings,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<SettingsView> {
    let old = state.db.get_settings().map_err(err)?;
    settings.max_items = settings.max_items.max(0);
    settings.auto_delete_days = settings.auto_delete_days.max(0);
    settings.ignore_apps = settings
        .ignore_apps
        .iter()
        .map(|a| a.trim().to_lowercase())
        .filter(|a| !a.is_empty())
        .collect();
    settings.ignore_apps.sort();
    settings.ignore_apps.dedup();
    // Win+V is switched through its own command (it touches Explorer).
    settings.shortcut_mode = old.shortcut_mode.clone();
    settings.monitor_paused = old.monitor_paused;

    if settings.shortcut.trim() != old.shortcut.trim() && settings.shortcut_mode != "win_v" {
        if let Err(e) = crate::register_shortcut(&app, &settings) {
            let _ = crate::register_shortcut(&app, &old);
            return Err(e);
        }
    }
    if settings.launch_at_startup != old.launch_at_startup {
        let autolaunch = app.autolaunch();
        let res = if settings.launch_at_startup {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
        res.map_err(|e| format!("Démarrage automatique : {e}"))?;
    }
    state.db.set_settings(&settings).map_err(err)?;
    if settings.max_items > 0
        && settings.max_items != old.max_items
        && state.db.enforce_limit(settings.max_items).map_err(err)? > 0
    {
        changed(&app);
    }
    settings_changed(&app);
    Ok(settings_view(&state, settings))
}

#[tauri::command]
pub async fn set_api_key(provider: String, key: String) -> CmdResult<bool> {
    credentials::set(&provider, &key)?;
    Ok(credentials::is_set(&provider))
}

/// The window edges follow the theme the interface resolved.
#[tauri::command]
pub fn set_frame_theme(light: bool, webview_window: tauri::WebviewWindow) {
    crate::window::style_frame(&webview_window, light);
}

// ─── Win+V ───

/// Take over Win+V: tell Explorer to release it, optionally restart Explorer,
/// then register it. Returns whether Win+V now opens Clipper.
#[tauri::command]
pub async fn enable_win_v(
    restart_explorer: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<bool> {
    hotkey::set_explorer_win_v_disabled(true)?;
    let mut settings = state.db.get_settings().map_err(err)?;
    settings.shortcut_mode = "win_v".into();
    state.db.set_settings(&settings).map_err(err)?;
    if restart_explorer {
        tauri::async_runtime::spawn_blocking(hotkey::restart_explorer)
            .await
            .map_err(err)??;
        // Explorer needs a moment before the key can be claimed.
        for _ in 0..20 {
            std::thread::sleep(Duration::from_millis(250));
            if crate::register_shortcut(&app, &settings).is_ok() {
                settings_changed(&app);
                return Ok(true);
            }
        }
        settings_changed(&app);
        return Ok(false);
    }
    let ok = crate::register_shortcut(&app, &settings).is_ok();
    settings_changed(&app);
    Ok(ok)
}

/// Give Win+V back to Windows and use the custom shortcut again.
#[tauri::command]
pub async fn disable_win_v(
    restart_explorer: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<()> {
    let mut settings = state.db.get_settings().map_err(err)?;
    settings.shortcut_mode = "custom".into();
    state.db.set_settings(&settings).map_err(err)?;
    let _ = crate::register_shortcut(&app, &settings);
    hotkey::set_explorer_win_v_disabled(false)?;
    if restart_explorer {
        tauri::async_runtime::spawn_blocking(hotkey::restart_explorer)
            .await
            .map_err(err)??;
    }
    settings_changed(&app);
    Ok(())
}

// ─── Capture ───

/// `minutes`: None resumes, Some(0) pauses until resumed, Some(n) for n minutes.
#[tauri::command]
pub fn set_incognito(minutes: Option<u64>, app: AppHandle) {
    match minutes {
        None => tray::set_paused(&app, false, None),
        Some(0) => tray::set_paused(&app, true, None),
        Some(n) => tray::set_paused(&app, true, Some(Duration::from_secs(n * 60))),
    }
}

// ─── Windows ───

#[tauri::command]
pub fn show_popup(app: AppHandle) {
    window::show_popup(&app);
}

#[tauri::command]
pub fn hide_popup(app: AppHandle) {
    window::hide_popup(&app);
}

#[tauri::command]
pub fn show_main(app: AppHandle) {
    window::hide_popup(&app);
    window::show_main(&app);
}

#[tauri::command]
pub fn hide_main(app: AppHandle) {
    if let Some(w) = app.get_webview_window(window::MAIN) {
        let _ = w.hide();
    }
}

#[tauri::command]
pub async fn complete_onboarding(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<SettingsView> {
    let mut settings = state.db.get_settings().map_err(err)?;
    settings.onboarded = true;
    state.db.set_settings(&settings).map_err(err)?;
    settings_changed(&app);
    Ok(settings_view(&state, settings))
}

// ─── Backups ───

#[tauri::command]
pub async fn list_backups(state: State<'_, AppState>) -> CmdResult<Vec<BackupInfo>> {
    Ok(backup::list(&state.db))
}

#[tauri::command]
pub async fn backup_now(state: State<'_, AppState>) -> CmdResult<Vec<BackupInfo>> {
    backup::run_now(&state.db).map_err(err)?;
    Ok(backup::list(&state.db))
}

#[tauri::command]
pub async fn restore_backup(
    name: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<()> {
    backup::restore(&state.db, &name)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn open_data_folder(state: State<'_, AppState>) -> CmdResult<()> {
    tauri_plugin_opener::open_path(state.db.data_dir(), None::<&str>).map_err(err)
}
