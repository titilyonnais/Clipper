use super::{changed, err, AppState, CmdResult};
use crate::clipboard;
use crate::db::{self, Db, ImportedClip};
use crate::files::{self, FileInfo};
use crate::models::{ImportResult, KINDS};
use base64::Engine;
use std::collections::HashMap;
use tauri::ipc::Response;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

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
        return Err("Par sécurité, les programmes et scripts ne sont pas lancés depuis Clipper. Utilisez « Afficher dans le dossier ».".into());
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
    let (written, skipped) = tauri::async_runtime::spawn_blocking(move || write_export(&db, &path))
        .await
        .map_err(err)??;
    Ok(Some(if skipped > 0 {
        format!("{written} élément(s) exporté(s). {skipped} élément(s) sensible(s) non exporté(s).")
    } else {
        format!("{written} élément(s) exporté(s).")
    }))
}

/// Write the export clip by clip (images are read one at a time), leaving
/// out sensitive clips: an export is a plain file that may travel.
/// Returns (exported, skipped).
fn write_export(db: &Db, path: &std::path::Path) -> CmdResult<(usize, usize)> {
    use std::io::Write;
    let collections: HashMap<i64, String> = db
        .collections()
        .map_err(err)?
        .into_iter()
        .map(|c| (c.id, c.name))
        .collect();
    let rows = db.export_rows().map_err(err)?;
    let mut out = std::io::BufWriter::new(std::fs::File::create(path).map_err(err)?);
    write!(
        out,
        r#"{{"app":"clipper","version":3,"exported_at":{},"clips":["#,
        serde_json::Value::from(db::now())
    )
    .map_err(err)?;
    let (mut written, mut skipped) = (0, 0);
    for c in rows {
        if c.sensitive {
            skipped += 1;
            continue;
        }
        // Images travel inside the export as base64 PNG.
        let content = if c.kind == "image" {
            let Some(png) = c.image_path.as_ref().and_then(|p| std::fs::read(p).ok()) else {
                continue;
            };
            base64::engine::general_purpose::STANDARD.encode(png)
        } else {
            c.content.unwrap_or_default()
        };
        let item = serde_json::json!({
            "kind": c.kind,
            "content": content,
            "language": c.language,
            "tags": c.tags,
            "pinned": c.pinned,
            "collection": c.collection_id.and_then(|id| collections.get(&id)),
            "source_app": c.source_app,
            "created_at": c.created_at,
            "used_at": c.used_at,
            "use_count": c.use_count,
        });
        if written > 0 {
            out.write_all(b",").map_err(err)?;
        }
        serde_json::to_writer(&mut out, &item).map_err(err)?;
        written += 1;
    }
    out.write_all(b"]}").map_err(err)?;
    out.flush().map_err(err)?;
    Ok((written, skipped))
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

/// Accepts exports from Clipper 1, 2 and 3.
pub fn import_file(db: &Db, path: &std::path::Path) -> CmdResult<ImportResult> {
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
    let str_of = |c: &Value, k: &str| {
        c[k].as_str()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };
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
        let (content, hash, preview, size, sensitive) = match kind {
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
                    false,
                )
            }
            "file" => {
                let Ok(paths) = serde_json::from_str::<Vec<String>>(content) else {
                    continue;
                };
                if paths.is_empty() || !paths.iter().all(|p| files::is_local_path(p)) {
                    continue;
                }
                (
                    content.to_string(),
                    clipboard::hash_files(&paths),
                    clipboard::files_preview(&paths),
                    content.len() as i64,
                    false,
                )
            }
            _ => (
                content.to_string(),
                clipboard::hash_text(content),
                clipboard::make_preview(content),
                content.len() as i64,
                c["sensitive"].as_bool().unwrap_or(false)
                    || crate::sensitive::detect(content).is_some(),
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
            tags: db::normalize_tags(&tags),
            pinned: c["pinned"].as_bool().unwrap_or(false)
                || c["favorite"].as_bool().unwrap_or(false),
            collection: str_of(c, "collection")
                .or_else(|| str_of(c, "category"))
                .map(|s| s.chars().take(48).collect()),
            source_app: str_of(c, "source_app"),
            size_bytes: size,
            hash,
            sensitive,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{NewClip, RichFormats};

    fn add(db: &Db, text: &str, sensitive: bool) {
        db.upsert_clip(&NewClip {
            kind: "text",
            content: text,
            preview: text,
            language: None,
            source_app: None,
            size_bytes: text.len() as i64,
            hash: &clipboard::hash_text(text),
            sensitive,
            rich: &RichFormats::default(),
        })
        .unwrap();
    }

    #[test]
    fn export_leaves_secrets_out_and_imports_back() {
        let dir = std::env::temp_dir().join(format!("clipper-export-{}", std::process::id()));
        let (a, b) = (dir.join("a"), dir.join("b"));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        let source = Db::open(&a).unwrap();
        add(&source, "compte rendu", false);
        add(&source, "sk-ant-secret", true);
        let file = dir.join("export.json");
        assert_eq!(write_export(&source, &file).unwrap(), (1, 1));
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(!text.contains("sk-ant-secret"));

        let target = Db::open(&b).unwrap();
        assert_eq!(import_file(&target, &file).unwrap().imported, 1);
        drop((source, target));
        std::fs::remove_dir_all(dir).ok();
    }
}
