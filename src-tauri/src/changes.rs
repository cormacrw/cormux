//! The Changes panel: which diff it shows for a workspace, keeping it live, and discarding work.

use std::path::Path;

use crate::error::Result;
use crate::git::DiffTarget;
use crate::model::WorkspaceKind;
use crate::state::AppState;
use crate::workspaces::WorkspaceRecord;

/// Keeps the workspace's diff live. A PR review shows the branch against its base unless the
/// user picks something else.
pub fn watch(state: &AppState, workspace: &WorkspaceRecord) -> Result<()> {
    if workspace.kind == WorkspaceKind::PrReview {
        state.diffs.default_diff_target(
            &workspace.id,
            DiffTarget::head_against(workspace.base.clone()),
        );
    }
    let worktree = Path::new(&workspace.worktree_path);
    if worktree.exists() {
        state.diffs.watch(&workspace.id, worktree)?;
    }
    Ok(())
}

/// A workspace from an earlier launch has no watcher until something asks for its diff.
pub async fn load(state: &AppState, workspace_id: &str) -> Result<WorkspaceRecord> {
    let workspace = state.workspaces.get(workspace_id).await?;
    watch(state, &workspace)?;
    Ok(workspace)
}

pub async fn refresh(state: &AppState, workspace_id: &str) -> Result<()> {
    let workspace = load(state, workspace_id).await?;
    state
        .diffs
        .compute(workspace_id, Path::new(&workspace.worktree_path))
        .await?;
    Ok(())
}

/// Points the panel at uncommitted work (`None`) or a branch against its base.
pub async fn set_target(
    state: &AppState,
    workspace_id: &str,
    target: Option<DiffTarget>,
) -> Result<()> {
    let workspace = load(state, workspace_id).await?;
    state.diffs.set_diff_target(workspace_id, target);
    state
        .diffs
        .compute(workspace_id, Path::new(&workspace.worktree_path))
        .await?;
    Ok(())
}

/// Throws away uncommitted changes to `paths` (all of them when empty).
pub async fn discard(state: &AppState, workspace_id: &str, paths: &[String]) -> Result<()> {
    let workspace = load(state, workspace_id).await?;
    let worktree = Path::new(&workspace.worktree_path);
    state.git.discard(worktree, paths).await?;
    state.diffs.compute(workspace_id, worktree).await?;
    Ok(())
}

pub mod commands {
    use tauri::State;

    use crate::error::Result;
    use crate::git::DiffTarget;
    use crate::state::AppState;

    /// Re-reads the workspace's changes now, after something outside the watcher moved them.
    #[tauri::command]
    #[specta::specta]
    pub async fn refresh_workspace_diff(
        workspace_id: String,
        state: State<'_, AppState>,
    ) -> Result<()> {
        super::refresh(&state, &workspace_id).await
    }

    /// Point the Changes panel at uncommitted work (`None`) or a branch against its base.
    #[tauri::command]
    #[specta::specta]
    pub async fn set_workspace_diff_target(
        workspace_id: String,
        target: Option<DiffTarget>,
        state: State<'_, AppState>,
    ) -> Result<()> {
        super::set_target(&state, &workspace_id, target).await
    }

    /// Throw away uncommitted changes to `paths` (all of them when empty), then refresh Changes.
    #[tauri::command]
    #[specta::specta]
    pub async fn discard_workspace_changes(
        workspace_id: String,
        paths: Vec<String>,
        state: State<'_, AppState>,
    ) -> Result<()> {
        super::discard(&state, &workspace_id, &paths).await
    }
}
