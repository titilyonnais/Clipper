use super::{changed, err, AppState, CmdResult};
use crate::models::{Collection, Snippet};
use tauri::{AppHandle, State, WebviewWindow};

// ─── Collections ───

#[tauri::command]
pub async fn list_collections(state: State<'_, AppState>) -> CmdResult<Vec<Collection>> {
    state.db.collections().map_err(err)
}

#[tauri::command]
pub async fn create_collection(
    name: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<i64> {
    let id = state.db.create_collection(&name).map_err(err)?;
    changed(&app);
    Ok(id)
}

#[tauri::command]
pub async fn rename_collection(
    id: i64,
    name: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<()> {
    state.db.rename_collection(id, &name).map_err(err)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn delete_collection(
    id: i64,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<()> {
    state.db.delete_collection(id).map_err(err)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn reorder_collections(
    ids: Vec<i64>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<()> {
    state.db.reorder_collections(&ids).map_err(err)?;
    changed(&app);
    Ok(())
}

/// File clips in a collection (`None` removes them from their collection).
#[tauri::command]
pub async fn set_collection(
    ids: Vec<i64>,
    collection_id: Option<i64>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<()> {
    for id in ids {
        state.db.set_collection(id, collection_id).map_err(err)?;
    }
    changed(&app);
    Ok(())
}

// ─── Tags ───

#[tauri::command]
pub async fn list_tags(state: State<'_, AppState>) -> CmdResult<Vec<(String, i64)>> {
    state.db.tags().map_err(err)
}

#[tauri::command]
pub async fn update_tags(
    id: i64,
    tags: Vec<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<()> {
    state.db.update_tags(id, &tags).map_err(err)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn rename_tag(
    old: String,
    new: String,
    app: AppHandle,
    state: State<'_, AppState>,
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
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<usize> {
    let n = state.db.edit_tag(&name, None).map_err(err)?;
    changed(&app);
    Ok(n)
}

// ─── Snippets ───

#[tauri::command]
pub async fn list_snippets(
    query: Option<String>,
    state: State<'_, AppState>,
) -> CmdResult<Vec<Snippet>> {
    state.db.snippets(query.as_deref()).map_err(err)
}

#[tauri::command]
pub async fn save_snippet(
    snippet: Snippet,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<i64> {
    let id = state.db.save_snippet(&snippet).map_err(err)?;
    let _ = tauri::Emitter::emit(&app, "snippets:changed", ());
    Ok(id)
}

#[tauri::command]
pub async fn delete_snippet(id: i64, app: AppHandle, state: State<'_, AppState>) -> CmdResult<()> {
    state.db.delete_snippet(id).map_err(err)?;
    let _ = tauri::Emitter::emit(&app, "snippets:changed", ());
    Ok(())
}

/// Save a clip as a new snippet.
#[tauri::command]
pub async fn clip_to_snippet(
    id: i64,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<i64> {
    let (kind, content) = state.db.content(id).map_err(err)?;
    if kind == "image" || kind == "file" {
        return Err("Seul le texte peut devenir un snippet.".into());
    }
    let title: String = content
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("Snippet")
        .trim()
        .chars()
        .take(60)
        .collect();
    let id = state
        .db
        .save_snippet(&Snippet {
            id: 0,
            title,
            abbreviation: None,
            content,
            use_count: 0,
            updated_at: String::new(),
        })
        .map_err(err)?;
    let _ = tauri::Emitter::emit(&app, "snippets:changed", ());
    Ok(id)
}

/// Expand a snippet's variables and paste (popup) or copy (main window) it.
#[tauri::command]
pub async fn paste_snippet(
    id: i64,
    shift_held: bool,
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> CmdResult<super::clips::PasteOutcome> {
    let snippet = state.db.snippet(id).map_err(err)?;
    let text = crate::template::expand(&snippet.content, crate::clipboard::read_text);
    state.db.bump_snippet(id).map_err(err)?;
    super::clips::paste_text(text, shift_held, app, window, state).await
}
