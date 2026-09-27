mod ai;
mod clipboard;
mod commands;
mod db;
mod files;
mod models;
mod secrets;

use crate::clipboard::MonitorState;
use crate::commands::AppState;
use crate::db::Db;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

const TRAY_ID: &str = "main";

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_main(app)
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            // CLIPPER_DATA_DIR allows a portable install or a throw-away test profile.
            let data_dir = match std::env::var_os("CLIPPER_DATA_DIR") {
                Some(dir) => std::path::PathBuf::from(dir),
                None => app.path().app_data_dir()?,
            };
            std::fs::create_dir_all(&data_dir)?;
            init_log(&data_dir);
            let db = Arc::new(Db::open(&data_dir)?);
            // The web view may only load images from Clipper's own image folder.
            app.asset_protocol_scope()
                .allow_directory(data_dir.join("images"), false)?;

            for (provider, key) in db.take_legacy_api_keys().unwrap_or_default() {
                if let Err(e) = secrets::set(provider, &key) {
                    log::warn!("could not move {provider} key to the credential store: {e}");
                }
            }

            let mut settings = db.get_settings().unwrap_or_default();
            // The OS is the source of truth for autostart: if the user disabled
            // it elsewhere, follow that instead of re-registering ourselves.
            if let Ok(enabled) = app.autolaunch().is_enabled() {
                if enabled != settings.launch_at_startup {
                    settings.launch_at_startup = enabled;
                    let _ = db.set_settings(&settings);
                }
            }

            let monitor = Arc::new(MonitorState::default());
            monitor
                .paused
                .store(settings.monitor_paused, Ordering::Relaxed);
            clipboard::spawn_monitor(app.handle().clone(), db.clone(), monitor.clone());
            app.manage(AppState {
                db: db.clone(),
                monitor,
            });

            build_tray(app.handle(), settings.monitor_paused)?;
            if let Err(e) = register_shortcut(app.handle(), &settings.shortcut) {
                log::warn!("{e}");
            }
            spawn_cleanup(app.handle().clone(), db);

            if !std::env::args().any(|a| a == "--minimized") {
                show_main(app.handle());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_clips,
            commands::get_clip,
            commands::copy_clip,
            commands::toggle_pin,
            commands::toggle_favorite,
            commands::update_tags,
            commands::update_category,
            commands::delete_clip,
            commands::clear_history,
            commands::cleanup_now,
            commands::file_infos,
            commands::file_preview,
            commands::open_file,
            commands::reveal_file,
            commands::open_url,
            commands::export_history,
            commands::import_history,
            commands::get_stats,
            commands::get_histogram,
            commands::list_categories,
            commands::list_tags,
            commands::list_languages,
            commands::category_counts,
            commands::tag_counts,
            commands::rename_category,
            commands::delete_category,
            commands::rename_tag,
            commands::delete_tag,
            commands::get_settings,
            commands::set_settings,
            commands::set_api_key,
            commands::set_paused,
            commands::hide_window,
            commands::ai_health,
            commands::ai_run,
            commands::ai_smart_tag,
        ])
        .on_window_event(|window, event| {
            // Closing the window keeps Clipper running in the notification area.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Clipper");
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

/// Hourly retention cleanup (first pass shortly after start-up).
fn spawn_cleanup(app: AppHandle, db: Arc<Db>) {
    std::thread::Builder::new()
        .name("retention".into())
        .spawn(move || {
            std::thread::sleep(Duration::from_secs(15));
            loop {
                let s = db.get_settings().unwrap_or_default();
                match db.cleanup_expired(s.auto_delete_days, s.keep_favorites, s.keep_pinned) {
                    Ok(n) if n > 0 => {
                        let _ = app.emit("clips:changed", ());
                    }
                    Err(e) => log::warn!("retention cleanup failed: {e}"),
                    _ => {}
                }
                std::thread::sleep(Duration::from_secs(3600));
            }
        })
        .expect("failed to start retention thread");
}

fn tray_tooltip(paused: bool) -> &'static str {
    if paused {
        "Clipper (capture en pause)"
    } else {
        "Clipper"
    }
}

fn build_tray(app: &AppHandle, paused: bool) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Ouvrir Clipper", true, None::<&str>)?;
    let pause = MenuItem::with_id(
        app,
        "pause",
        "Suspendre / reprendre la capture",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quitter", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &pause, &quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(tray_tooltip(paused))
        .icon(
            app.default_window_icon()
                .cloned()
                .ok_or(tauri::Error::InvalidIcon(std::io::Error::other(
                    "missing default icon",
                )))?,
        )
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "pause" => {
                if let Some(state) = app.try_state::<AppState>() {
                    let paused = !commands::is_paused(&state);
                    if let Ok(mut s) = state.db.get_settings() {
                        s.monitor_paused = paused;
                        let _ = state.db.set_settings(&s);
                    }
                    apply_paused(app, paused);
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// Apply the paused state everywhere: monitor, tray and UI.
pub fn apply_paused(app: &AppHandle, paused: bool) {
    if let Some(state) = app.try_state::<AppState>() {
        state.monitor.paused.store(paused, Ordering::Relaxed);
    }
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(tray_tooltip(paused)));
    }
    let _ = app.emit("monitor:paused", paused);
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        let _ = app.emit("window:shown", ());
    }
}

fn toggle_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let visible = w.is_visible().unwrap_or(false);
        let focused = w.is_focused().unwrap_or(false);
        if visible && focused {
            let _ = w.hide();
        } else {
            show_main(app);
        }
    }
}

/// Register the global show/hide shortcut. An empty accelerator disables it.
pub fn register_shortcut(app: &AppHandle, accelerator: &str) -> Result<(), String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let accel = accelerator.trim();
    if accel.is_empty() {
        return Ok(());
    }
    let shortcut: Shortcut = accel
        .parse()
        .map_err(|_| format!("Raccourci invalide : {accel}"))?;
    gs.on_shortcut(shortcut, |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            toggle_main(app);
        }
    })
    .map_err(|_| format!("Le raccourci {accel} est déjà utilisé par une autre application."))
}
