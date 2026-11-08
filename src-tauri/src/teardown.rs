use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::feedback::Change;
use crate::model::WorkspaceLifecycle;
use crate::state::AppState;
use crate::workspaces::WorkspaceRecord;

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TeardownDataLoss {
    pub uncommitted_files: u32,
    pub unpushed_commits: u32,
    pub has_data_loss: bool,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TeardownPreview {
    pub workspace_id: String,
    pub workspace_name: String,
    pub engine_label: String,
    pub branch: String,
    pub worktree_path: String,
    pub app_running: bool,
    pub delete_branch_default: bool,
    pub data_loss: TeardownDataLoss,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TeardownInput {
    pub workspace_id: String,
    pub delete_branch: bool,
}

pub async fn preview(state: &AppState, workspace_id: &str) -> Result<TeardownPreview> {
    let record = state.workspaces.get(workspace_id).await?;
    let row = state
        .store
        .workspace_by_id(workspace_id)?
        .ok_or_else(|| Error::unknown_workspace(workspace_id))?;
    let worktree = PathBuf::from(&record.worktree_path);

    let uncommitted_files = count_uncommitted_files(state, &worktree).await;
    let unpushed_commits = if worktree.is_dir() {
        state
            .git
            .commits_only_here(&worktree, &record.base)
            .await
            // Unknown counts as at risk: the default then keeps the branch.
            .unwrap_or(1)
    } else {
        0
    };
    let has_data_loss = uncommitted_files > 0 || unpushed_commits > 0;
    let warning = data_loss_warning(uncommitted_files, unpushed_commits);

    let branch = record.branch.clone();
    Ok(TeardownPreview {
        workspace_id: workspace_id.to_string(),
        workspace_name: record.name.clone(),
        engine_label: "Claude Code".into(),
        branch: branch.clone(),
        worktree_path: record.worktree_path.clone(),
        app_running: state.process.workspace_has_session(workspace_id),
        // A branch holding the only copy of its commits stays, so a later thread can pick it up.
        delete_branch_default: row.pr_number.is_none() && unpushed_commits == 0,
        data_loss: TeardownDataLoss {
            uncommitted_files,
            unpushed_commits,
            has_data_loss,
            warning,
        },
    })
}

/// Stops everything running in the workspace, removes its worktree (and branch, if asked) and
/// archives it. Setup still running stops too: it checks the workspace is alive between steps,
/// and its command's process is stopped with the rest.
pub async fn execute(state: &AppState, input: &TeardownInput) -> Result<()> {
    let record = state.workspaces.get(&input.workspace_id).await?;
    if record.status == WorkspaceLifecycle::TearingDown {
        return Err(Error::Workspace(format!(
            "workspace {} is already being torn down",
            input.workspace_id
        )));
    }
    state
        .workspaces
        .set_status(&input.workspace_id, WorkspaceLifecycle::TearingDown)
        .await?;
    state.out.changed(Change::THREADS);

    let thread_ids: Vec<String> = state
        .store
        .threads_for_workspace(&input.workspace_id)?
        .into_iter()
        .map(|thread| thread.id)
        .collect();

    if let Err(error) = run_teardown_steps(state, &record, input.delete_branch, &thread_ids).await {
        let restored = state
            .workspaces
            .set_status(&input.workspace_id, after_failed_teardown(record.status))
            .await;
        if let Err(restore) = restored {
            log::warn!("restoring {} after teardown: {restore}", input.workspace_id);
        }
        state.out.changed(Change::THREADS);
        return Err(error);
    }

    state.workspaces.archive(&input.workspace_id).await?;
    state.diffs.unwatch(&input.workspace_id);
    state.process.forget_workspace(&input.workspace_id);
    state.workspace_git.forget(&input.workspace_id).await;

    state.out.changed(Change::THREADS);

    Ok(())
}

async fn run_teardown_steps(
    state: &AppState,
    record: &WorkspaceRecord,
    delete_branch: bool,
    thread_ids: &[String],
) -> Result<()> {
    state.engines.stop_threads(thread_ids).await?;
    state.process.stop_workspace_sessions(&record.id).await;

    let repo = PathBuf::from(&record.repo_path);
    let worktree = PathBuf::from(&record.worktree_path);
    if worktree.exists() {
        let force = state
            .git
            .status_porcelain(&worktree)
            .await
            .map(|status| !status.trim().is_empty())
            .unwrap_or(false);
        state.git.worktree_remove(&repo, &worktree, force).await?;
    }

    if delete_branch {
        state.git.branch_delete(&repo, &record.branch).await?;
    }

    Ok(())
}

/// What a workspace is once a teardown that stopped its agents and setup gives up.
fn after_failed_teardown(before: WorkspaceLifecycle) -> WorkspaceLifecycle {
    match before {
        WorkspaceLifecycle::Running => WorkspaceLifecycle::Idle,
        WorkspaceLifecycle::Provisioning => WorkspaceLifecycle::ProvisioningFailed,
        other => other,
    }
}

async fn count_uncommitted_files(state: &AppState, worktree: &Path) -> u32 {
    if !worktree.is_dir() {
        return 0;
    }
    match state.git.numstat(worktree).await {
        Ok(files) => files.len() as u32,
        Err(_) => 0,
    }
}

fn data_loss_warning(uncommitted_files: u32, unpushed_commits: u32) -> Option<String> {
    let mut parts = Vec::new();
    if uncommitted_files > 0 {
        let label = if uncommitted_files == 1 {
            "1 uncommitted file change".into()
        } else {
            format!("{uncommitted_files} uncommitted file changes")
        };
        parts.push(format!("{label} will be discarded."));
    }
    if unpushed_commits > 0 {
        let label = if unpushed_commits == 1 {
            "1 unpushed commit".into()
        } else {
            format!("{unpushed_commits} unpushed commits")
        };
        parts.push(format!("{label} will be lost if you delete the branch."));
    }
    if parts.is_empty() {
        return None;
    }
    Some(format!("{} This can't be undone.", parts.join(" ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ThreadRole, WorkspaceKind};
    use crate::store::types::{RepoRecord, ThreadRow, WorkspaceRow};

    /// Teardown used to leave a workspace mid-setup marked as provisioning, so setup carried on
    /// into the folder being deleted.
    #[tokio::test]
    async fn a_workspace_mid_setup_is_torn_down_and_its_setup_told() {
        let (state, _recording) =
            AppState::recorded(crate::paths::DataDir::new(std::env::temp_dir()));
        state
            .store
            .upsert_repo(&RepoRecord {
                id: "r1".into(),
                path: "/tmp/app".into(),
                name: "app".into(),
                default_branch: Some("main".into()),
                setup_commands: String::new(),
                run_command: None,
                single_instance: false,
            })
            .unwrap();
        let row = WorkspaceRow::new(
            WorkspaceKind::Lead,
            "w",
            "r1",
            "Work",
            "feat/x",
            "main",
            "/tmp/cormux-no-such-worktree",
        );
        let repo = state.store.repo_by_id("r1").unwrap().unwrap();
        state
            .workspaces
            .register(
                row,
                &ThreadRow::new("t", "w", ThreadRole::Lead, "claude"),
                &repo,
            )
            .await
            .unwrap();

        execute(
            &state,
            &TeardownInput {
                workspace_id: "w".into(),
                delete_branch: false,
            },
        )
        .await
        .unwrap();

        assert!(state.workspaces.get("w").await.is_err());
        let archived = state.store.workspace_by_id("w").unwrap().unwrap();
        assert!(archived.archived_at.is_some());
        assert_eq!(archived.status, WorkspaceLifecycle::TearingDown);
    }

    #[test]
    fn a_failed_teardown_hands_back_what_it_stopped() {
        assert_eq!(
            after_failed_teardown(WorkspaceLifecycle::Provisioning),
            WorkspaceLifecycle::ProvisioningFailed
        );
        assert_eq!(
            after_failed_teardown(WorkspaceLifecycle::Running),
            WorkspaceLifecycle::Idle
        );
        assert_eq!(
            after_failed_teardown(WorkspaceLifecycle::Ready),
            WorkspaceLifecycle::Ready
        );
    }

    #[test]
    fn data_loss_message_covers_uncommitted_and_unpushed() {
        let message = data_loss_warning(2, 3).unwrap();
        assert!(message.contains("2 uncommitted file changes"));
        assert!(message.contains("3 unpushed commits"));
    }

    #[test]
    fn no_warning_when_nothing_is_at_risk() {
        assert!(data_loss_warning(0, 0).is_none());
    }
}
