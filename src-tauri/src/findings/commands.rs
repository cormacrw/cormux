use tauri::State;

use crate::error::Result;
use crate::state::AppState;

use super::{DismissWorkspaceFindingsInput, SendWorkspaceFindingsInput};

#[tauri::command]
#[specta::specta]
pub async fn send_workspace_findings(
    input: SendWorkspaceFindingsInput,
    state: State<'_, AppState>,
) -> Result<()> {
    super::send_workspace_findings(
        &state,
        &input.workspace_id,
        &input.thread_id,
        input.finding_ids,
    )
    .await
}

/// Dismissed findings stay listed but aren't sent to an agent or posted with the review.
#[tauri::command]
#[specta::specta]
pub async fn dismiss_workspace_findings(
    input: DismissWorkspaceFindingsInput,
    state: State<'_, AppState>,
) -> Result<()> {
    super::dismiss_findings(&state, input)
}
