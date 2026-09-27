use crate::ai;
use crate::clipboard::{self, MonitorState};
use crate::db::{self, Db, ImportedClip};
use crate::files::{self, FileInfo};
use crate::models::{
    AiResponse, ClipItem, ImportResult, ListParams, Settings, SettingsView, Stats, KINDS,
};
use crate::secrets;
use base64::Engine;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::ipc::Response;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_dialog::DialogExt;

pub struct AppState {
    pub db: Arc<Db>,
    pub monitor: Arc<MonitorState>,
}

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn changed(app: &AppHandle) {
    let _ = app.emit("clips:changed", ());
}

#[tauri::command]
pub async fn list_clips(
    params: ListParams,
    state: State<'_, AppState>,
) -> CmdResult<Vec<ClipItem>> {
    state.db.list(&params).map_err(err)
}

#[tauri::command]
pub async fn get_clip(id: i64, state: State<'_, AppState>) -> CmdResult<Option<ClipItem>> {
    state.db.get(id).map_err(err)
}

/// Put a clip back on the clipboard, optionally transformed.
#[tauri::command]
pub async fn copy_clip(
    id: i64,
    format: Option<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> CmdResult<()> {
    let (kind, content) = state.db.content(id).map_err(err)?;
    let monitor = &state.monitor;
    match (kind.as_str(), format.as_deref()) {
        ("image", None) => {
            let png = std::fs::read(state.db.image_path(&content))
                .map_err(|_| "Le fichier image est introuvable.".to_string())?;
            clipboard::write_png(monitor, png).map_err(err)?;
        }
        ("file", None) => {
            let paths: Vec<String> = serde_json::from_str(&content).map_err(err)?;
            clipboard::write_files(monitor, &paths).map_err(err)?;
        }
        ("image" | "file", Some(_)) => return Err("Format non applicable à ce type.".into()),
        (_, fmt) => {
            let text = transform(&content, fmt.unwrap_or("plain"))?;
            clipboard::write_text(monitor, &text).map_err(err)?;
        }
    }
    state.db.bump_used(id).map_err(err)?;
    changed(&app);
    Ok(())
}

fn transform(s: &str, format: &str) -> CmdResult<String> {
    Ok(match format {
        "plain" => s.to_string(),
        "trim" => s.trim().to_string(),
        "lowercase" => s.to_lowercase(),
        "uppercase" => s.to_uppercase(),
        "json_escape" => serde_json::to_string(s).map_err(err)?,
        "url_encode" => s
            .bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    (b as char).to_string()
                }
                _ => format!("%{b:02X}"),
            })
            .collect(),
        "base64" => base64::engine::general_purpose::STANDARD.encode(s.as_bytes()),
        _ => return Err("Format inconnu.".into()),
    })
}

#[tauri::command]
pub async fn toggle_pin(id: i64, state: State<'_, AppState>, app: AppHandle) -> CmdResult<()> {
    state.db.toggle_pin(id).map_err(err)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn toggle_favorite(id: i64, state: State<'_, AppState>, app: AppHandle) -> CmdResult<()> {
    state.db.toggle_favorite(id).map_err(err)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn update_tags(
    id: i64,
    tags: Vec<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> CmdResult<()> {
    state.db.update_tags(id, &tags).map_err(err)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn update_category(
    id: i64,
    category: Option<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> CmdResult<()> {
    state
        .db
        .update_category(id, category.as_deref())
        .map_err(err)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn delete_clip(id: i64, state: State<'_, AppState>, app: AppHandle) -> CmdResult<()> {
    state.db.delete(id).map_err(err)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn clear_history(state: State<'_, AppState>, app: AppHandle) -> CmdResult<usize> {
    let keep_pinned = state.db.get_settings().map_err(err)?.keep_pinned;
    let n = state.db.clear_all(keep_pinned).map_err(err)?;
    changed(&app);
    Ok(n)
}

#[tauri::command]
pub async fn cleanup_now(state: State<'_, AppState>, app: AppHandle) -> CmdResult<usize> {
    let s = state.db.get_settings().map_err(err)?;
    let n = state
        .db
        .cleanup_expired(s.auto_delete_days, s.keep_favorites, s.keep_pinned)
        .map_err(err)?;
    changed(&app);
    Ok(n)
}

// ─── File clips: every path comes from the database, never from the web view ───

fn clip_paths(db: &Db, id: i64) -> CmdResult<Vec<String>> {
    let (kind, content) = db.content(id).map_err(err)?;
    if kind != "file" {
        return Err("Cet élément ne contient pas de fichiers.".into());
    }
    serde_json::from_str(&content).map_err(err)
}

fn clip_path(db: &Db, id: i64, index: usize) -> CmdResult<String> {
    let path = clip_paths(db, id)?
        .into_iter()
        .nth(index)
        .ok_or("Fichier introuvable.")?;
    if !std::path::Path::new(&path).exists() {
        return Err("Le fichier n'existe plus à cet emplacement.".into());
    }
    Ok(path)
}

#[tauri::command]
pub async fn file_infos(id: i64, state: State<'_, AppState>) -> CmdResult<Vec<FileInfo>> {
    Ok(clip_paths(&state.db, id)?
        .iter()
        .map(|p| files::info(p))
        .collect())
}

/// Raw bytes of an image file referenced by a file clip (for previews).
#[tauri::command]
pub async fn file_preview(
    id: i64,
    index: usize,
    state: State<'_, AppState>,
) -> CmdResult<Response> {
    let path = clip_path(&state.db, id, index)?;
    let info = files::info(&path);
    if !info.is_image {
        return Err("Aperçu indisponible pour ce format.".into());
    }
    if info.size > files::MAX_PREVIEW_BYTES {
        return Err("Image trop volumineuse pour l'aperçu.".into());
    }
    Ok(Response::new(std::fs::read(&path).map_err(err)?))
}

#[tauri::command]
pub async fn open_file(id: i64, index: usize, state: State<'_, AppState>) -> CmdResult<()> {
    let path = clip_path(&state.db, id, index)?;
    if files::is_executable(&path) {
        return Err("Par sécurité, les programmes et scripts ne sont pas lancés depuis Clipper. Utilisez « Localiser ».".into());
    }
    tauri_plugin_opener::open_path(&path, None::<&str>).map_err(err)
}

#[tauri::command]
pub async fn reveal_file(id: i64, index: usize, state: State<'_, AppState>) -> CmdResult<()> {
    let path = clip_path(&state.db, id, index)?;
    tauri_plugin_opener::reveal_item_in_dir(&path).map_err(err)
}

#[tauri::command]
pub async fn open_url(id: i64, state: State<'_, AppState>) -> CmdResult<()> {
    let (kind, url) = state.db.content(id).map_err(err)?;
    let url = url.trim();
    let lower = url.to_ascii_lowercase();
    let allowed = ["http://", "https://", "mailto:"]
        .iter()
        .any(|p| lower.starts_with(p));
    if kind != "url" || !allowed || url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("Lien non pris en charge.".into());
    }
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(err)
}

// ─── Export / import (file chosen by the user through the native dialog) ───

#[tauri::command]
pub async fn export_history(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<Option<String>> {
    let name = format!(
        "clipper-export-{}.json",
        chrono::Local::now().format("%Y-%m-%d")
    );
    let dialog = app
        .dialog()
        .file()
        .add_filter("JSON", &["json"])
        .set_file_name(name);
    let Some(path) = tauri::async_runtime::spawn_blocking(move || dialog.blocking_save_file())
        .await
        .map_err(err)?
    else {
        return Ok(None);
    };
    let path = path.into_path().map_err(err)?;
    let db = state.db.clone();
    let written = tauri::async_runtime::spawn_blocking(move || -> CmdResult<usize> {
        let mut clips = db.export_rows().map_err(err)?;
        for c in clips.iter_mut().filter(|c| c.kind == "image") {
            // Images travel inside the export as base64 PNG.
            let png = c
                .image_path
                .as_ref()
                .and_then(|p| std::fs::read(p).ok())
                .unwrap_or_default();
            c.content = Some(base64::engine::general_purpose::STANDARD.encode(png));
            c.image_path = None;
        }
        let payload = serde_json::json!({
            "app": "clipper",
            "version": 2,
            "exported_at": db::now(),
            "clips": clips,
        });
        let file = std::fs::File::create(&path).map_err(err)?;
        serde_json::to_writer(std::io::BufWriter::new(file), &payload).map_err(err)?;
        Ok(clips.len())
    })
    .await
    .map_err(err)??;
    Ok(Some(format!("{written} élément(s) exporté(s).")))
}

const MAX_IMPORT_BYTES: u64 = 1024 * 1024 * 1024;

#[tauri::command]
pub async fn import_history(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<Option<ImportResult>> {
    let dialog = app.dialog().file().add_filter("JSON", &["json"]);
    let Some(path) = tauri::async_runtime::spawn_blocking(move || dialog.blocking_pick_file())
        .await
        .map_err(err)?
    else {
        return Ok(None);
    };
    let path = path.into_path().map_err(err)?;
    let db = state.db.clone();
    let result = tauri::async_runtime::spawn_blocking(move || import_file(&db, &path))
        .await
        .map_err(err)??;
    changed(&app);
    Ok(Some(result))
}

fn import_file(db: &Db, path: &std::path::Path) -> CmdResult<ImportResult> {
    use serde_json::Value;
    if std::fs::metadata(path).map_err(err)?.len() > MAX_IMPORT_BYTES {
        return Err("Fichier trop volumineux.".into());
    }
    let file = std::fs::File::open(path).map_err(err)?;
    let v: Value = serde_json::from_reader(std::io::BufReader::new(file))
        .map_err(|e| format!("JSON invalide : {e}"))?;
    let items = v["clips"]
        .as_array()
        .ok_or("Ce fichier n'est pas un export Clipper.")?;
    let total = items.len();
    let now = db::now();
    let str_of = |c: &Value, k: &str| c[k].as_str().map(str::to_string).filter(|s| !s.is_empty());
    let date_of = |c: &Value, k: &str| {
        c[k].as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|d| {
                d.with_timezone(&chrono::Utc)
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            })
    };

    let mut clips = Vec::with_capacity(total);
    for c in items {
        let kind = c["kind"].as_str().unwrap_or("");
        let Some(content) = c["content"].as_str().filter(|s| !s.is_empty()) else {
            continue;
        };
        if !KINDS.contains(&kind) {
            continue;
        }
        // Hashes are recomputed: an imported file can never impersonate another clip.
        let (content, hash, preview, size) = match kind {
            "image" => {
                let Ok(png) = base64::engine::general_purpose::STANDARD.decode(content.trim())
                else {
                    continue;
                };
                let Some((w, h)) = clipboard::png_dimensions(&png) else {
                    continue;
                };
                let Ok(file) = db.store_image_file(&png) else {
                    continue;
                };
                (
                    file,
                    clipboard::hash_image(&png),
                    format!("{w}×{h}"),
                    png.len() as i64,
                )
            }
            "file" => {
                let Ok(paths) = serde_json::from_str::<Vec<String>>(content) else {
                    continue;
                };
                (
                    content.to_string(),
                    clipboard::hash_files(&paths),
                    clipboard::files_preview(&paths),
                    content.len() as i64,
                )
            }
            _ => (
                content.to_string(),
                clipboard::hash_text(content),
                clipboard::make_preview(content),
                content.len() as i64,
            ),
        };
        let tags: Vec<String> = c["tags"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|t| t.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let created_at = date_of(c, "created_at").unwrap_or_else(|| now.clone());
        clips.push(ImportedClip {
            kind: kind.to_string(),
            content,
            preview,
            language: str_of(c, "language").filter(|_| kind == "code"),
            category: str_of(c, "category").map(|s| s.chars().take(64).collect()),
            tags: db::normalize_tags(&tags),
            pinned: c["pinned"].as_bool().unwrap_or(false),
            favorite: c["favorite"].as_bool().unwrap_or(false),
            source_app: str_of(c, "source_app"),
            size_bytes: size,
            hash,
            used_at: date_of(c, "used_at").unwrap_or_else(|| created_at.clone()),
            created_at,
            use_count: c["use_count"].as_i64().unwrap_or(1).max(1),
        });
    }
    let imported = db.import(&clips).map_err(err)?;
    Ok(ImportResult {
        imported,
        skipped: total - imported,
        total,
    })
}

// ─── Aggregates ───

#[tauri::command]
pub async fn get_stats(state: State<'_, AppState>) -> CmdResult<Stats> {
    state.db.stats().map_err(err)
}

#[tauri::command]
pub async fn get_histogram(days: i64, state: State<'_, AppState>) -> CmdResult<Vec<(String, i64)>> {
    state.db.histogram(days).map_err(err)
}

#[tauri::command]
pub async fn list_categories(state: State<'_, AppState>) -> CmdResult<Vec<String>> {
    state.db.categories().map_err(err)
}

#[tauri::command]
pub async fn list_tags(state: State<'_, AppState>) -> CmdResult<Vec<String>> {
    state.db.tags().map_err(err)
}

#[tauri::command]
pub async fn list_languages(state: State<'_, AppState>) -> CmdResult<Vec<String>> {
    state.db.languages().map_err(err)
}

#[tauri::command]
pub async fn category_counts(state: State<'_, AppState>) -> CmdResult<Vec<(String, i64)>> {
    state.db.category_counts().map_err(err)
}

#[tauri::command]
pub async fn tag_counts(state: State<'_, AppState>) -> CmdResult<Vec<(String, i64)>> {
    state.db.tag_counts().map_err(err)
}

#[tauri::command]
pub async fn rename_category(
    old: String,
    new: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> CmdResult<usize> {
    let n = state.db.rename_category(&old, &new).map_err(err)?;
    changed(&app);
    Ok(n)
}

#[tauri::command]
pub async fn delete_category(
    name: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> CmdResult<usize> {
    let n = state.db.delete_category(&name).map_err(err)?;
    changed(&app);
    Ok(n)
}

#[tauri::command]
pub async fn rename_tag(
    old: String,
    new: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> CmdResult<usize> {
    if new.trim().trim_start_matches('#').is_empty() {
        return Err("Le nom ne peut pas être vide.".into());
    }
    let n = state.db.edit_tag(&old, Some(&new)).map_err(err)?;
    changed(&app);
    Ok(n)
}

#[tauri::command]
pub async fn delete_tag(
    name: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> CmdResult<usize> {
    let n = state.db.edit_tag(&name, None).map_err(err)?;
    changed(&app);
    Ok(n)
}

// ─── Settings ───

pub fn settings_view(db: &Db, settings: Settings) -> SettingsView {
    SettingsView {
        settings,
        openai_key_set: secrets::is_set("openai"),
        anthropic_key_set: secrets::is_set("anthropic"),
        data_dir: db.data_dir().to_string_lossy().into_owned(),
    }
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> CmdResult<SettingsView> {
    Ok(settings_view(
        &state.db,
        state.db.get_settings().map_err(err)?,
    ))
}

#[tauri::command]
pub async fn set_settings(
    mut settings: Settings,
    state: State<'_, AppState>,
    app: AppHandle,
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
    settings.ignore_apps.dedup();

    if settings.shortcut.trim() != old.shortcut.trim() {
        if let Err(e) = crate::register_shortcut(&app, &settings.shortcut) {
            let _ = crate::register_shortcut(&app, &old.shortcut);
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
    if settings.monitor_paused != old.monitor_paused {
        crate::apply_paused(&app, settings.monitor_paused);
    }
    state.db.set_settings(&settings).map_err(err)?;
    if settings.max_items > 0
        && settings.max_items != old.max_items
        && state.db.enforce_limit(settings.max_items).map_err(err)? > 0
    {
        changed(&app);
    }
    Ok(settings_view(&state.db, settings))
}

#[tauri::command]
pub async fn set_api_key(provider: String, key: String) -> CmdResult<bool> {
    secrets::set(&provider, &key)?;
    Ok(secrets::is_set(&provider))
}

#[tauri::command]
pub async fn set_paused(paused: bool, state: State<'_, AppState>, app: AppHandle) -> CmdResult<()> {
    let mut s = state.db.get_settings().map_err(err)?;
    s.monitor_paused = paused;
    state.db.set_settings(&s).map_err(err)?;
    crate::apply_paused(&app, paused);
    Ok(())
}

#[tauri::command]
pub fn hide_window(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

// ─── AI ───

#[tauri::command]
pub async fn ai_health(state: State<'_, AppState>) -> CmdResult<AiResponse> {
    let s = state.db.get_settings().map_err(err)?;
    Ok(ai::health(&s).await)
}

#[tauri::command]
pub async fn ai_run(
    id: i64,
    action: String,
    lang: Option<String>,
    state: State<'_, AppState>,
) -> CmdResult<AiResponse> {
    Ok(ai::run(&state.db, id, &action, lang.as_deref()).await)
}

#[tauri::command]
pub async fn ai_smart_tag(
    id: i64,
    state: State<'_, AppState>,
    app: AppHandle,
) -> CmdResult<AiResponse> {
    let res = ai::smart_tag(&state.db, id).await;
    if res.ok {
        changed(&app);
    }
    Ok(res)
}

pub fn is_paused(state: &AppState) -> bool {
    state.monitor.paused.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::transform;

    #[test]
    fn transforms() {
        assert_eq!(transform("  a b ", "trim").unwrap(), "a b");
        assert_eq!(transform("é/ &", "url_encode").unwrap(), "%C3%A9%2F%20%26");
        assert_eq!(transform("a\"b", "json_escape").unwrap(), "\"a\\\"b\"");
        assert_eq!(transform("hi", "base64").unwrap(), "aGk=");
        assert!(transform("x", "rot13").is_err());
    }
}
