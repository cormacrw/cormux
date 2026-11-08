//! The login-shell environment the app runs git, `gh` and agents in, and what it finds there.

use tauri::State;

use crate::engines::EngineStatus;
use crate::error::Result;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn reload_environment(state: State<'_, AppState>) -> Result<()> {
    state.shell_env.write().await.load().await?;
    state.out.changed();
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn detect_engines(state: State<'_, AppState>) -> Result<Vec<EngineStatus>> {
    let env = state.shell_env.read().await.clone();
    crate::engines::detect_engines(&env).await
}
