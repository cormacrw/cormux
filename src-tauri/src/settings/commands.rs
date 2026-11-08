use tauri::State;

use crate::error::Result;
use crate::feedback::Change;
use crate::state::AppState;

use super::{Settings, SettingsPatch};

#[tauri::command]
#[specta::specta]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings> {
    Ok(Settings::load(&state.store))
}

/// Saves the fields `patch` sets and returns every setting as now stored.
#[tauri::command]
#[specta::specta]
pub async fn update_settings(patch: SettingsPatch, state: State<'_, AppState>) -> Result<Settings> {
    Settings::update(&state.store, patch)?;
    state.out.changed(Change::SNAPSHOT);
    Ok(Settings::load(&state.store))
}
