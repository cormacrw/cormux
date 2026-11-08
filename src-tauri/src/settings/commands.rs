use tauri::State;

use crate::error::Result;
use crate::state::AppState;

use super::{Settings, SettingsPatch};

/// Saves the fields `patch` sets and returns every setting as now stored.
#[tauri::command]
#[specta::specta]
pub async fn update_settings(patch: SettingsPatch, state: State<'_, AppState>) -> Result<Settings> {
    Settings::update(&state.store, patch)?;
    state.out.changed();
    Settings::read(&state.store)
}
