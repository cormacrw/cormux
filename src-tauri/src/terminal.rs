//! Opening a workspace in the user's terminal app.

use crate::error::{Error, Result};
use crate::state::AppState;

/// Opens the worktree in the terminal app picked in Settings (Terminal by default).
pub async fn open(state: &AppState, workspace_id: &str) -> Result<()> {
    let workspace = state.workspaces.get(workspace_id).await?;
    let terminal = terminal_app(state.store.get_setting("terminalApp")?);
    let status = tokio::process::Command::new("open")
        .args(["-a", &terminal, &workspace.worktree_path])
        .status()
        .await
        .map_err(|error| Error::Process(format!("open {terminal}: {error}")))?;
    if !status.success() {
        return Err(Error::Process(format!(
            "{terminal} couldn't open {}",
            workspace.worktree_path
        )));
    }
    Ok(())
}

fn terminal_app(setting: Option<String>) -> String {
    setting
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Terminal".to_string())
}

/// Open the worktree in the terminal app picked in Settings (Terminal by default).
#[tauri::command]
#[specta::specta]
pub async fn open_workspace_terminal(
    workspace_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<()> {
    open(&state, &workspace_id).await
}
