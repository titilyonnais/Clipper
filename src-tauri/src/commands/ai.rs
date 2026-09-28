use super::{changed, err, AppState, CmdResult};
use crate::ai;
use crate::models::AiResponse;
use tauri::{AppHandle, State};

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
