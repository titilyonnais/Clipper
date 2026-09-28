#[cfg(not(windows))]
compile_error!("Clipper est une application Windows.");

mod ai;
mod appicons;
mod backup;
mod clipboard;
mod commands;
mod credentials;
mod db;
mod files;
mod hotkey;
mod models;
mod ocr;
mod paste;
mod sensitive;
mod template;
mod tray;
mod window;

use crate::commands::AppState;
use crate::db::Db;
use crate::models::Settings;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

static SHORTCUT_OK: AtomicBool = AtomicBool::new(false);

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            window::show_main(app)
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            setup(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::clips::list_clips,
            commands::clips::get_clip,
            commands::clips::get_stats,
            commands::clips::source_apps,
            commands::clips::paste_clip,
            commands::clips::paste_text,
            commands::clips::copy_transformed,
            commands::clips::update_clip_text,
            commands::clips::set_pinned,
            commands::clips::set_sensitive,
            commands::clips::delete_clips,
            commands::clips::undo_delete,
            commands::clips::clear_history,
            commands::clips::start_queue,
            commands::clips::stop_queue,
            commands::clips::queue_status,
            commands::organize::list_collections,
            commands::organize::create_collection,
            commands::organize::rename_collection,
            commands::organize::delete_collection,
            commands::organize::set_collection,
            commands::organize::update_tags,
            commands::organize::list_snippets,
            commands::organize::save_snippet,
            commands::organize::delete_snippet,
            commands::organize::clip_to_snippet,
            commands::organize::paste_snippet,
            commands::data::file_infos,
            commands::data::file_preview,
            commands::data::open_file,
            commands::data::reveal_file,
            commands::data::open_url,
            commands::data::export_history,
            commands::data::import_history,
            commands::system::get_settings,
            commands::system::set_settings,
            commands::system::set_api_key,
            commands::system::set_frame_theme,
            commands::system::enable_win_v,
            commands::system::disable_win_v,
            commands::system::set_incognito,
            commands::system::show_popup,
            commands::system::hide_popup,
            commands::system::show_main,
            commands::system::hide_main,
            commands::system::complete_onboarding,
            commands::system::list_backups,
            commands::system::backup_now,
            commands::system::restore_backup,
            commands::system::open_data_folder,
            commands::ai::ai_health,
            commands::ai::ai_run,
            commands::ai::ai_smart_tag,
        ])
        .on_window_event(|window, event| match event {
            // Closing the main window keeps Clipper running in the notification area.
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            // The popup disappears as soon as the user clicks elsewhere.
            tauri::WindowEvent::Focused(false) if window.label() == window::POPUP => {
                let _ = window.hide();
            }
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while building Clipper")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                clipboard::queue::stop_and_wait();
                if let Some(state) = app.try_state::<AppState>() {
                    if let Err(e) = state.db.forget_undo() {
                        log::warn!("forget_undo: {e}");
                    }
                    state.db.checkpoint();
                }
            }
        });
}

fn setup(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // CLIPPER_DATA_DIR allows a portable install or a throw-away test profile.
    let data_dir = match std::env::var_os("CLIPPER_DATA_DIR") {
        Some(dir) => std::path::PathBuf::from(dir),
        None => app.path().app_data_dir()?,
    };
    std::fs::create_dir_all(&data_dir)?;
    init_log(&data_dir);
    let db = Arc::new(Db::open(&data_dir)?);
    let icons_dir = data_dir.join("icons");
    // The web view may only load images from Clipper's own folders.
    let scope = app.asset_protocol_scope();
    scope.allow_directory(data_dir.join("images"), false)?;
    scope.allow_directory(&icons_dir, false)?;

    for (provider, key) in db.take_legacy_api_keys().unwrap_or_default() {
        if let Err(e) = credentials::set(provider, &key) {
            log::warn!("could not move {provider} key to the credential store: {e}");
        }
    }

    let mut settings = db.get_settings().unwrap_or_default();
    // The OS is the source of truth for autostart: if the user disabled it
    // elsewhere, follow that instead of re-registering ourselves.
    if let Ok(enabled) = app.autolaunch().is_enabled() {
        settings.launch_at_startup = enabled;
    }
    let _ = db.set_settings(&settings);

    let monitor = Arc::new(clipboard::MonitorState::default());
    if settings.monitor_paused {
        monitor.pause(None);
    }

    let handle = app.clone();
    let ocr = ocr::Ocr::start(db.clone(), move |id| {
        let _ = handle.emit("clip:ocr", id);
    });

    let handle = app.clone();
    let icon_dir = icons_dir.clone();
    clipboard::spawn_monitor(db.clone(), monitor.clone(), move |stored| {
        let _ = handle.emit("clips:changed", ());
        if stored.kind == "image" {
            if let Some(state) = handle.try_state::<AppState>() {
                state.ocr.enqueue(stored.id);
            }
        }
        if let Some(path) = stored.app_path {
            let dir = icon_dir.clone();
            std::thread::spawn(move || appicons::ensure_icon(&dir, &path));
        }
    });

    let handle = app.clone();
    clipboard::queue::init(move |status| {
        let _ = handle.emit("queue:changed", status);
    });

    app.manage(AppState {
        db: db.clone(),
        monitor,
        ocr,
    });

    if let Some(main) = app.get_webview_window(window::MAIN) {
        window::style_frame(&main, false);
    }
    window::create_popup(app)?;
    tray::build(app)?;
    // Keep Explorer's setting (and the uninstaller's marker) in line with the mode.
    if settings.shortcut_mode == "win_v" {
        if let Err(e) = hotkey::set_explorer_win_v_disabled(true) {
            log::warn!("{e}");
        }
    }
    if let Err(e) = register_shortcut(app, &settings) {
        log::warn!("{e}");
    }
    spawn_maintenance(app.clone(), db);

    if !std::env::args().any(|a| a == "--minimized") || !settings.onboarded {
        window::show_main(app);
    }
    Ok(())
}

/// Hourly retention cleanup and daily backup (first pass shortly after start-up).
fn spawn_maintenance(app: AppHandle, db: Arc<Db>) {
    std::thread::Builder::new()
        .name("maintenance".into())
        .spawn(move || {
            std::thread::sleep(Duration::from_secs(30));
            loop {
                let s = db.get_settings().unwrap_or_default();
                // Undo is offered for a few seconds only.
                if let Err(e) = db.forget_undo() {
                    log::warn!("forget_undo: {e}");
                }
                match db.cleanup_expired(s.auto_delete_days) {
                    Ok(n) if n > 0 => {
                        let _ = app.emit("clips:changed", ());
                    }
                    Err(e) => log::warn!("retention cleanup failed: {e}"),
                    _ => {}
                }
                if s.backups_enabled {
                    if let Err(e) = backup::run_daily(&db) {
                        log::warn!("backup failed: {e}");
                    }
                }
                std::thread::sleep(Duration::from_secs(3600));
            }
        })
        .expect("failed to start maintenance thread");
}

/// Whether the last shortcut registration succeeded.
pub fn shortcut_registered() -> bool {
    SHORTCUT_OK.load(Ordering::Relaxed)
}

/// Register the global shortcut that opens the popup: Win+V, or the custom
/// accelerator (empty = none). When Win+V is still held by Explorer, the
/// custom shortcut is registered instead so the popup stays reachable.
pub fn register_shortcut(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    SHORTCUT_OK.store(false, Ordering::Relaxed);
    let custom = settings.shortcut.trim();
    let register = |accel: &str| -> Result<(), String> {
        let shortcut: Shortcut = accel
            .parse()
            .map_err(|_| format!("Raccourci invalide : {accel}"))?;
        gs.on_shortcut(shortcut, |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                window::toggle_popup(app);
            }
        })
        .map_err(|_| format!("Le raccourci {accel} est déjà utilisé par une autre application."))
    };
    if settings.shortcut_mode == "win_v" {
        if register("Super+V").is_ok() {
            SHORTCUT_OK.store(true, Ordering::Relaxed);
            return Ok(());
        }
        if !custom.is_empty() {
            let _ = register(custom);
        }
        return Err("Win+V est encore réservé par Windows : relancez l'Explorateur.".into());
    }
    if custom.is_empty() {
        return Ok(());
    }
    register(custom)?;
    SHORTCUT_OK.store(true, Ordering::Relaxed);
    Ok(())
}

/// Optional diagnostics: set CLIPPER_LOG=1 (or =debug) to write clipper.log
/// in the data folder. Nothing is logged otherwise.
fn init_log(data_dir: &std::path::Path) {
    struct FileLog(parking_lot::Mutex<std::fs::File>);
    impl log::Log for FileLog {
        fn enabled(&self, _: &log::Metadata) -> bool {
            true
        }
        fn log(&self, r: &log::Record) {
            if r.target().starts_with("clipper") {
                use std::io::Write;
                let _ = writeln!(self.0.lock(), "{} {:5} {}", db::now(), r.level(), r.args());
            }
        }
        fn flush(&self) {}
    }
    let Some(level) = std::env::var("CLIPPER_LOG").ok() else {
        return;
    };
    let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(data_dir.join("clipper.log"))
    else {
        return;
    };
    if log::set_boxed_logger(Box::new(FileLog(parking_lot::Mutex::new(file)))).is_ok() {
        log::set_max_level(if level == "debug" {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        });
    }
}
