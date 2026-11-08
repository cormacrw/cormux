use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::model::{ThreadStatus, WorkspaceLifecycle};
use crate::state::AppState;
use crate::workspaces::WorkspaceRecord;

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TeardownDataLoss {
    /// The worktree teardown deletes has uncommitted changes, or git couldn't say, so the user
    /// confirms first. Unpushed commits alone don't ask: the branch is then kept by default.
    pub ask_first: bool,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TeardownPreview {
    pub workspace_id: String,
    pub workspace_name: String,
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

    // A count git couldn't give is at risk: the warning says so and the branch is kept.
    let uncommitted = count_uncommitted_files(state, &worktree).await;
    let unpushed = if worktree.is_dir() {
        match state.git.commits_only_here(&worktree, &record.base).await {
            Ok(count) => Some(count),
            Err(error) => {
                log::warn!("counting {workspace_id}'s unpushed commits: {error}");
                None
            }
        }
    } else {
        Some(0)
    };

    let branch = record.branch.clone();
    Ok(TeardownPreview {
        workspace_id: workspace_id.to_string(),
        workspace_name: record.name.clone(),
        branch: branch.clone(),
        worktree_path: record.worktree_path.clone(),
        app_running: state.process.workspace_has_session(workspace_id),
        // A branch holding the only copy of its commits stays, so a later thread can pick it up.
        delete_branch_default: row.pr_number.is_none() && unpushed == Some(0),
        data_loss: TeardownDataLoss {
            ask_first: uncommitted != Some(0),
            warning: data_loss_warning(uncommitted, unpushed),
        },
    })
}

/// Setup still running stops too: it checks the workspace is alive between steps, and its
/// command's process is stopped with the rest.
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
    state.out.changed();

    let thread_ids: Vec<String> = state
        .store
        .threads_for_workspace(&input.workspace_id)?
        .into_iter()
        .map(|thread| thread.id)
        .collect();

    if let Err(error) = run_teardown_steps(state, &record, input.delete_branch, &thread_ids).await {
        let restored = settle_stopped_threads(state, &input.workspace_id).await;
        let restored = match restored {
            Ok(()) => state
                .workspaces
                .set_status(&input.workspace_id, after_failed_teardown(record.status))
                .await
                .map(drop),
            Err(error) => Err(error),
        };
        if let Err(restore) = restored {
            log::warn!("restoring {} after teardown: {restore}", input.workspace_id);
        }
        state.out.changed();
        return Err(error);
    }

    state.workspaces.archive(&input.workspace_id).await?;
    state.diffs.unwatch(&input.workspace_id);
    state.process.forget_workspace(&input.workspace_id);
    state.workspace_git.forget(&input.workspace_id).await;

    state.out.changed();

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

/// The engines teardown stopped report no turn end, so a thread they left running is idled
/// here. A paused one stays paused: resuming launches a new engine.
async fn settle_stopped_threads(state: &AppState, workspace_id: &str) -> Result<()> {
    for thread in state.store.threads_for_workspace(workspace_id)? {
        if thread.status == ThreadStatus::Running {
            state
                .workspaces
                .set_thread_status(&thread.id, workspace_id, ThreadStatus::Idle)
                .await?;
        }
    }
    Ok(())
}

/// What a workspace is once a teardown that stopped its agents and setup gives up.
fn after_failed_teardown(before: WorkspaceLifecycle) -> WorkspaceLifecycle {
    match before {
        WorkspaceLifecycle::Running | WorkspaceLifecycle::Paused => WorkspaceLifecycle::Idle,
        WorkspaceLifecycle::Provisioning => WorkspaceLifecycle::ProvisioningFailed,
        other => other,
    }
}

async fn count_uncommitted_files(state: &AppState, worktree: &Path) -> Option<u32> {
    if !worktree.is_dir() {
        return Some(0);
    }
    match state.git.numstat(worktree).await {
        Ok(files) => Some(files.len() as u32),
        Err(error) => {
            log::warn!(
                "counting uncommitted files in {}: {error}",
                worktree.display()
            );
            None
        }
    }
}

/// `None` is a count git couldn't give.
fn data_loss_warning(uncommitted: Option<u32>, unpushed: Option<u32>) -> Option<String> {
    let mut parts = Vec::new();
    let uncommitted_files = match uncommitted {
        Some(count) => count,
        None => {
            parts.push("Couldn't tell whether the worktree has uncommitted changes; any will be discarded.".to_string());
            0
        }
    };
    let unpushed_commits = match unpushed {
        Some(count) => count,
        None => {
            parts.push(
                "Couldn't tell whether the branch has commits found nowhere else.".to_string(),
            );
            0
        }
    };
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

pub mod commands {
    use tauri::State;

    use crate::error::Result;
    use crate::state::AppState;

    use super::{TeardownInput, TeardownPreview};

    #[tauri::command]
    #[specta::specta]
    pub async fn get_teardown_preview(
        workspace_id: String,
        state: State<'_, AppState>,
    ) -> Result<TeardownPreview> {
        super::preview(&state, &workspace_id).await
    }

    #[tauri::command]
    #[specta::specta]
    pub async fn teardown_workspace(
        input: TeardownInput,
        state: State<'_, AppState>,
    ) -> Result<()> {
        super::execute(&state, &input).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ThreadRole, WorkspaceKind};
    use crate::store::types::{RepoRecord, ThreadRow, WorkspaceRow};

    async fn registered(worktree: &str) -> AppState {
        let (state, _recording) =
            AppState::recorded(crate::paths::DataDir::new(std::env::temp_dir()));
        state
            .store
            .upsert_repo(&RepoRecord {
                path: "/tmp/app".into(),
                name: "app".into(),
                ..crate::store::fixtures::repo("r1")
            })
            .unwrap();
        let row = WorkspaceRow::new(
            WorkspaceKind::Lead,
            "w",
            "r1",
            "Work",
            "feat/x",
            "main",
            worktree,
        );
        let repo = state.store.repo_by_id("r1").unwrap().unwrap();
        state
            .workspaces
            .register(
                row,
                &ThreadRow::new("t", "w", ThreadRole::Lead, "claude"),
                &Default::default(),
                &repo,
            )
            .await
            .unwrap();
        state
    }

    /// Teardown used to leave a workspace mid-setup marked as provisioning, so setup carried on
    /// into the folder being deleted.
    #[tokio::test]
    async fn a_workspace_mid_setup_is_torn_down_and_its_setup_told() {
        let state = registered("/tmp/cormux-no-such-worktree").await;

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

    /// Bug: when git couldn't count the uncommitted files the preview sent 0, and the dialog
    /// tore the worktree down without asking.
    #[tokio::test]
    async fn a_worktree_git_cannot_read_is_asked_about_first() {
        let folder = tempfile::tempdir().unwrap();
        let worktree = folder.path().join("not-a-repo");
        let state = registered(worktree.to_str().unwrap()).await;
        std::fs::create_dir(&worktree).unwrap();

        let preview = preview(&state, "w").await.unwrap();

        assert!(preview.data_loss.ask_first);
        let warning = preview.data_loss.warning.unwrap();
        assert!(warning.contains("uncommitted changes"), "{warning}");
        assert!(!preview.delete_branch_default);

        let state = registered("/tmp/cormux-no-such-worktree").await;
        let preview = super::preview(&state, "w").await.unwrap();
        assert!(!preview.data_loss.ask_first);
    }

    #[tokio::test]
    async fn a_failed_teardown_idles_the_threads_whose_engines_it_stopped() {
        let folder = tempfile::tempdir().unwrap();
        let worktree = folder.path().join("not-a-repo");
        let state = registered(worktree.to_str().unwrap()).await;
        std::fs::create_dir(&worktree).unwrap();
        state
            .workspaces
            .set_thread_status("t", "w", ThreadStatus::Running)
            .await
            .unwrap();

        let torn = execute(
            &state,
            &TeardownInput {
                workspace_id: "w".into(),
                delete_branch: false,
            },
        )
        .await;

        assert!(torn.is_err(), "not a git worktree, so it can't be removed");
        let thread = state.store.thread_by_id("t").unwrap().unwrap();
        assert_eq!(thread.status, ThreadStatus::Idle);
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
        let message = data_loss_warning(Some(2), Some(3)).unwrap();
        assert!(message.contains("2 uncommitted file changes"));
        assert!(message.contains("3 unpushed commits"));
    }

    #[test]
    fn no_warning_when_nothing_is_at_risk() {
        assert!(data_loss_warning(Some(0), Some(0)).is_none());
    }

    #[test]
    fn a_count_git_couldnt_give_is_at_risk() {
        let unknown_files = data_loss_warning(None, Some(0)).unwrap();
        assert!(
            unknown_files.contains("uncommitted changes"),
            "{unknown_files}"
        );
        let unknown_commits = data_loss_warning(Some(0), None).unwrap();
        assert!(unknown_commits.contains("commits"), "{unknown_commits}");
    }
}
