use super::{changed, err, AppState, CmdResult};
use crate::clipboard::{self, queue, Payload};
use crate::models::{ClipItem, ListParams, SourceApp, Stats};
use crate::window::{self, POPUP};
use base64::Engine;
use serde::Serialize;
use tauri::{AppHandle, State, WebviewWindow};

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

#[tauri::command]
pub async fn get_stats(state: State<'_, AppState>) -> CmdResult<Stats> {
    state.db.stats().map_err(err)
}

#[tauri::command]
pub async fn source_apps(state: State<'_, AppState>) -> CmdResult<Vec<SourceApp>> {
    state.db.source_apps().map_err(err)
}

/// Put a stored clip on the clipboard. `plain` drops the formatting.
fn put_clip_on_clipboard(state: &AppState, id: i64, plain: bool) -> CmdResult<()> {
    let (kind, content) = state.db.content(id).map_err(err)?;
    match kind.as_str() {
        "image" => {
            let png = std::fs::read(state.db.image_path(&content))
                .map_err(|_| "Le fichier image est introuvable.".to_string())?;
            clipboard::write(&state.monitor, Payload::Png(&png)).map_err(err)?;
        }
        "file" => {
            let paths: Vec<String> = serde_json::from_str(&content).map_err(err)?;
            clipboard::write(&state.monitor, Payload::Files(&paths)).map_err(err)?;
        }
        _ => {
            let keep_rich = !plain && !state.db.get_settings().map_err(err)?.always_plain_text;
            let rich = if keep_rich {
                Some(state.db.rich_formats(id).map_err(err)?)
            } else {
                None
            };
            clipboard::write(
                &state.monitor,
                Payload::Text {
                    text: &content,
                    rich: rich.as_ref().filter(|r| !r.is_empty()),
                },
            )
            .map_err(err)?;
        }
    }
    state.db.bump_used(id).map_err(err)
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PasteOutcome {
    /// Pasted into the application that was active before the popup.
    Pasted,
    /// Only copied (main window, direct paste disabled, or paste refused).
    Copied,
}

/// From the popup with direct paste enabled: hide the popup and paste into
/// the previous application. Elsewhere: the data just stays on the clipboard.
fn finish_paste(
    app: &AppHandle,
    window: &WebviewWindow,
    state: &AppState,
    shift_held: bool,
) -> CmdResult<PasteOutcome> {
    changed(app);
    let direct = state.db.get_settings().map_err(err)?.paste_directly;
    // The popup stays open on an error, so that its message is seen.
    if window.label() != POPUP || !direct || !crate::paste::can_paste()? {
        return Ok(PasteOutcome::Copied);
    }
    // Focus must be handed over while Clipper still owns it; hide afterwards.
    let result = crate::paste::paste_into_target(shift_held);
    window::hide_popup(app);
    result.map(|_| PasteOutcome::Pasted)
}

#[tauri::command]
pub async fn paste_clip(
    id: i64,
    plain: bool,
    shift_held: bool,
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> CmdResult<PasteOutcome> {
    put_clip_on_clipboard(&state, id, plain)?;
    finish_paste(&app, &window, &state, shift_held)
}

/// Paste (or copy) arbitrary text: an edited clip, an AI answer, a snippet.
#[tauri::command]
pub async fn paste_text(
    text: String,
    shift_held: bool,
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> CmdResult<PasteOutcome> {
    clipboard::write(
        &state.monitor,
        Payload::Text {
            text: &text,
            rich: None,
        },
    )
    .map_err(err)?;
    finish_paste(&app, &window, &state, shift_held)
}

/// Copy a text clip transformed (main window "Copier en…").
#[tauri::command]
pub async fn copy_transformed(
    id: i64,
    format: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<()> {
    let (kind, content) = state.db.content(id).map_err(err)?;
    if kind == "image" || kind == "file" {
        return Err("Format non applicable à ce type.".into());
    }
    let text = transform(&content, &format)?;
    clipboard::write(
        &state.monitor,
        Payload::Text {
            text: &text,
            rich: None,
        },
    )
    .map_err(err)?;
    state.db.bump_used(id).map_err(err)?;
    changed(&app);
    Ok(())
}

pub fn transform(s: &str, format: &str) -> CmdResult<String> {
    Ok(match format {
        "trim" => s
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string(),
        "lowercase" => s.to_lowercase(),
        "uppercase" => s.to_uppercase(),
        "one_line" => s.split_whitespace().collect::<Vec<_>>().join(" "),
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

/// Save edited text into the clip. Returns the id of the resulting clip
/// (another one if the new text already existed).
#[tauri::command]
pub async fn update_clip_text(
    id: i64,
    text: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<i64> {
    if text.trim().is_empty() {
        return Err("Le texte est vide.".into());
    }
    let (kind, _) = state.db.content(id).map_err(err)?;
    if kind == "image" || kind == "file" {
        return Err("Seul le texte peut être modifié.".into());
    }
    let sensitive = state.db.get_settings().map_err(err)?.detect_secrets
        && crate::sensitive::detect(&text).is_some();
    let id = state.db.update_text(id, &text, sensitive).map_err(err)?;
    changed(&app);
    Ok(id)
}

#[tauri::command]
pub async fn set_pinned(
    ids: Vec<i64>,
    pinned: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<()> {
    for id in ids {
        state.db.set_pinned(id, pinned).map_err(err)?;
    }
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn set_sensitive(
    id: i64,
    sensitive: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<()> {
    state.db.set_sensitive(id, sensitive).map_err(err)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn delete_clips(
    ids: Vec<i64>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<usize> {
    let n = state.db.delete(&ids).map_err(err)?;
    changed(&app);
    Ok(n)
}

#[tauri::command]
pub async fn undo_delete(app: AppHandle, state: State<'_, AppState>) -> CmdResult<usize> {
    let n = state.db.undo_delete().map_err(err)?;
    changed(&app);
    Ok(n)
}

#[tauri::command]
pub async fn clear_history(app: AppHandle, state: State<'_, AppState>) -> CmdResult<usize> {
    let n = state.db.clear_history().map_err(err)?;
    changed(&app);
    Ok(n)
}

// ─── Paste queue ───

#[tauri::command]
pub async fn start_queue(
    ids: Vec<i64>,
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> CmdResult<()> {
    let mut items = Vec::with_capacity(ids.len());
    for id in ids {
        let (kind, content) = state.db.content(id).map_err(err)?;
        if kind != "image" && kind != "file" {
            items.push(content);
        }
    }
    queue::start(items)?;
    if window.label() == POPUP {
        window::hide_popup(&app);
    }
    Ok(())
}

#[tauri::command]
pub fn stop_queue() {
    queue::stop();
}

#[tauri::command]
pub fn queue_status() -> queue::QueueStatus {
    queue::status()
}

#[cfg(test)]
mod tests {
    use super::transform;

    #[test]
    fn transforms() {
        assert_eq!(transform("  a b  \n c  ", "trim").unwrap(), "a b\n c");
        assert_eq!(transform("a\n  b\tc", "one_line").unwrap(), "a b c");
        assert_eq!(transform("é/ &", "url_encode").unwrap(), "%C3%A9%2F%20%26");
        assert_eq!(transform("a\"b", "json_escape").unwrap(), "\"a\\\"b\"");
        assert_eq!(transform("hi", "base64").unwrap(), "aGk=");
        assert!(transform("x", "rot13").is_err());
    }
}
