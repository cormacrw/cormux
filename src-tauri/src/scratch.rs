//! Scratches: one-off conversations from before every thread had its own worktree. They can't
//! be started or continued any more; what's left stays readable until it's deleted.

use tauri::AppHandle;

use crate::error::{Error, Result};
use crate::ipc::types::StateChangeKind;
use crate::state::AppState;

/// Discards the conversation. The repo is never touched.
pub async fn end(app: &AppHandle, state: &AppState, scratch_id: &str) -> Result<()> {
    if state.store.scratch_by_id(scratch_id)?.is_none() {
        return Err(Error::Workspace(format!("unknown scratch {scratch_id}")));
    }
    state.store.delete_scratch(scratch_id)?;
    state.notify(app, StateChangeKind::WorkspaceStatus);
    Ok(())
}
