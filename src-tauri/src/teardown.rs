use std::path::{Path, PathBuf};

use crate::app::WorkspaceAppStatus;
use crate::error::Result;
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
    let pr_number = state.workspaces.pr_number(workspace_id).await?;
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
        app_running: matches!(
            state.apps.runtime(workspace_id).status,
            WorkspaceAppStatus::Starting | WorkspaceAppStatus::Running
        ),
        // A branch holding the only copy of its commits stays, so a later thread can pick it up.
        // One whose worktree was never made may not be this workspace's to delete.
        delete_branch_default: pr_number.is_none() && unpushed == Some(0) && worktree.is_dir(),
        data_loss: TeardownDataLoss {
            ask_first: uncommitted != Some(0),
            warning: data_loss_warning(uncommitted, unpushed),
        },
    })
}

/// Setup still running stops too: its command's process is stopped with the rest, and its run is
/// cancelled once any worktree it was adding is there to remove.
pub async fn execute(state: &AppState, input: &TeardownInput) -> Result<()> {
    let record = state.workspaces.begin_teardown(&input.workspace_id).await?;
    state.out.changed();

    let thread_ids: Vec<String> = state
        .store
        .threads_for_workspace(&input.workspace_id)?
        .into_iter()
        .map(|thread| thread.id)
        .collect();

    if let Err(error) = run_teardown_steps(state, &record, input.delete_branch, &thread_ids).await {
        let restored = crate::turn::settle_stopped(state, &thread_ids).await;
        let restored = match restored {
            Ok(()) => state
                .workspaces
                .hand_back(&input.workspace_id, record.status)
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
    state.forget_workspace(&input.workspace_id).await;

    state.out.changed();

    Ok(())
}

async fn run_teardown_steps(
    state: &AppState,
    record: &WorkspaceRecord,
    delete_branch: bool,
    thread_ids: &[String],
) -> Result<()> {
    crate::turn::stop_engines(state, thread_ids).await?;
    state.process.stop_workspace_sessions(&record.id).await;
    let _setup = state.setup.cancel(&record.id).await;

    let repo = PathBuf::from(&record.repo_path);
    let worktree = PathBuf::from(&record.worktree_path);
    if worktree.exists() {
        // A worktree git can't read is removed without --force, so git refuses it if it's dirty.
        let force = match state.git.status_porcelain(&worktree).await {
            Ok(status) => !status.trim().is_empty(),
            Err(error) => {
                log::warn!(
                    "reading {}'s changes before removing it: {error}",
                    record.id
                );
                false
            }
        };
        state.git.worktree_remove(&repo, &worktree, force).await?;
    }

    // Setup that failed early never made the branch, and there's nothing to delete.
    let branch_ref = format!("refs/heads/{}", record.branch);
    if delete_branch && state.git.ref_exists(&repo, &branch_ref).await? {
        state.git.branch_delete(&repo, &record.branch).await?;
    }

    Ok(())
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
    use crate::model::{ThreadRole, ThreadStatus, WorkspaceKind, WorkspaceLifecycle};
    use crate::store::types::{RepoRecord, ThreadRow, WorkspaceRow};

    async fn registered(worktree: &str) -> AppState {
        registered_in("/tmp/app", worktree).await
    }

    async fn registered_in(repo: &str, worktree: &str) -> AppState {
        let (state, _recording) =
            AppState::recorded(crate::paths::DataDir::new(std::env::temp_dir()));
        state
            .store
            .upsert_repo(&RepoRecord {
                path: repo.into(),
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
                "",
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
        state.stacks.remember_unstacked("w", "feat");

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
        assert!(!state.stacks.recently_unstacked("w", "feat"));
    }

    /// Bug: a workspace whose setup failed before making its worktree defaulted to deleting a
    /// branch that was never made, so every teardown failed on `branch -D` with no dialog to
    /// untick it.
    #[tokio::test]
    async fn a_workspace_whose_worktree_was_never_made_tears_down_and_keeps_branches_by_default() {
        let repo = tempfile::tempdir().unwrap();
        let git = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(repo.path())
                .output()
                .unwrap()
                .status;
            assert!(status.success(), "git {args:?}");
        };
        git(&["init", "-q", "-b", "main"]);
        let state = registered_in(
            repo.path().to_str().unwrap(),
            "/tmp/cormux-no-such-worktree",
        )
        .await;
        state
            .workspaces
            .set_status("w", WorkspaceLifecycle::ProvisioningFailed)
            .await
            .unwrap();

        let preview = preview(&state, "w").await.unwrap();
        assert!(!preview.delete_branch_default);
        assert!(!preview.data_loss.ask_first);

        execute(
            &state,
            &TeardownInput {
                workspace_id: "w".into(),
                delete_branch: true,
            },
        )
        .await
        .unwrap();
        let archived = state.store.workspace_by_id("w").unwrap().unwrap();
        assert!(archived.archived_at.is_some());
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

    /// Bug: any setup command or app run that had ever finished left a session record, so the
    /// dialog said it would stop "the running app" when none was running.
    #[tokio::test]
    async fn the_app_only_reads_as_running_while_it_runs() {
        let state = registered("/tmp/cormux-no-such-worktree").await;
        let exit = state
            .process
            .run_to_exit(
                crate::process::SessionId::of_workspace("w", "setup-0"),
                &crate::process::ShellCommand {
                    cmd: "true".into(),
                    cwd: std::env::temp_dir(),
                    env: Vec::new(),
                },
                std::time::Duration::from_secs(10),
                Some("w"),
            )
            .await
            .unwrap();
        assert_eq!(exit, crate::process::Exit::Code(0));

        assert!(!preview(&state, "w").await.unwrap().app_running);
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
