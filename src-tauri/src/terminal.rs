//! Opening a workspace in the user's terminal or editor app.

use crate::error::{Error, Result};
use crate::state::AppState;

/// Opens the worktree in the terminal app picked in Settings (Terminal by default).
pub async fn open(state: &AppState, workspace_id: &str) -> Result<()> {
    let settings = crate::settings::Settings::read(&state.store)?;
    open_worktree(
        state,
        workspace_id,
        &app_or(&settings.terminal_app, "Terminal"),
    )
    .await
}

/// Opens the worktree in the editor app picked in Settings (Zed by default).
pub async fn open_editor(state: &AppState, workspace_id: &str) -> Result<()> {
    let settings = crate::settings::Settings::read(&state.store)?;
    open_worktree(state, workspace_id, &app_or(&settings.editor_app, "Zed")).await
}

async fn open_worktree(state: &AppState, workspace_id: &str, app: &str) -> Result<()> {
    let workspace = state.workspaces.get(workspace_id).await?;
    let status = tokio::process::Command::new("open")
        .args(["-a", app, &workspace.worktree_path])
        .status()
        .await
        .map_err(|error| Error::Process(format!("open {app}: {error}")))?;
    if !status.success() {
        return Err(Error::Process(format!(
            "{app} couldn't open {}",
            workspace.worktree_path
        )));
    }
    Ok(())
}

/// The app named in Settings, or `fallback` when that's blank.
fn app_or(setting: &str, fallback: &str) -> String {
    match setting.trim() {
        "" => fallback.into(),
        name => name.into(),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn open_workspace_terminal(
    workspace_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<()> {
    open(&state, &workspace_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn open_workspace_editor(
    workspace_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<()> {
    open_editor(&state, &workspace_id).await
}

#[cfg(test)]
mod tests {
    use super::app_or;

    #[test]
    fn a_blank_setting_falls_back_to_the_default_app() {
        assert_eq!(app_or("  ", "Zed"), "Zed");
        assert_eq!(app_or(" Cursor ", "Zed"), "Cursor");
    }
}
