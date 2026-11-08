use tauri::State;

use crate::error::Result;
use crate::state::AppState;

use super::types::{ClickupBoard, ClickupOption, ClickupTaskDetail};

/// Checks the key with ClickUp, then saves it to `~/.cormux/credentials.json`.
#[tauri::command]
#[specta::specta]
pub async fn set_clickup_api_key(key: String, state: State<'_, AppState>) -> Result<()> {
    state.clickup.save_key(&key).await?;
    state.out.changed();
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn clear_clickup_api_key(state: State<'_, AppState>) -> Result<()> {
    state.clickup.clear_key()?;
    state.out.changed();
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn clickup_workspaces(state: State<'_, AppState>) -> Result<Vec<ClickupOption>> {
    state.clickup.workspaces().await
}

#[tauri::command]
#[specta::specta]
pub async fn clickup_spaces(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ClickupOption>> {
    state.clickup.spaces(&workspace_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn clickup_folders(
    space_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ClickupOption>> {
    state.clickup.folders(&space_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn clickup_board(state: State<'_, AppState>) -> Result<ClickupBoard> {
    state.clickup.chosen_board(&state.store).await
}

#[tauri::command]
#[specta::specta]
pub async fn clickup_task(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<ClickupTaskDetail> {
    state.clickup.task(&task_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn set_clickup_task_status(
    task_id: String,
    status: String,
    state: State<'_, AppState>,
) -> Result<()> {
    state.clickup.set_status(&task_id, &status).await
}

#[tauri::command]
#[specta::specta]
pub async fn set_clickup_task_points(
    task_id: String,
    points: f64,
    state: State<'_, AppState>,
) -> Result<()> {
    state.clickup.set_points(&task_id, points).await
}
