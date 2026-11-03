use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error::{Error, Result};
use crate::feedback::Output;
use crate::ipc::types::{
    GitConflictOperation, GitConflictState, RenameWorkspaceInput, StateChangeKind, ToastPart,
    ToastRaisedPayload, ToastTone, WorkspaceGitRuntime,
};
use crate::paths::expand_tilde;
use crate::state::AppState;
use crate::store::types::WorkspaceRow;
use crate::timeline::{BranchChange, TimelineStep, record_step};
use crate::workspace::{WorkspaceLifecycle, WorkspaceRecord};

/// Load a workspace persisted by an earlier launch and start its live diff watcher.
/// Only creating a worktree does either, so without this a restored workspace has
/// no diff until something else (an agent starting) loads it.
pub async fn load_workspace(state: &AppState, workspace_id: &str) -> Result<WorkspaceRecord> {
    let record = match state.workspace.get(workspace_id).await {
        Some(record) => record,
        None => {
            let row = state
                .store
                .workspace_by_id(workspace_id)?
                .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
            let record = resolve_record(state, workspace_id, &row).await?;
            state.workspace.remember(record.clone()).await;
            record
        }
    };
    if Path::new(&record.worktree_path).exists() {
        state
            .diffs
            .watch(workspace_id, Path::new(&record.worktree_path))?;
    }
    Ok(record)
}

/// Workspaces from earlier runs, so background fetches cover them before any is opened.
pub async fn restore_fetch_targets(state: &AppState) -> Result<()> {
    for row in state.store.live_workspaces()? {
        if state.workspace.get(&row.id).await.is_none() {
            let record = resolve_record(state, &row.id, &row).await?;
            state.workspace.remember(record).await;
        }
    }
    state.workspace.sync_fetch_targets().await;
    Ok(())
}

pub async fn resolve_record(
    state: &AppState,
    workspace_id: &str,
    row: &WorkspaceRow,
) -> Result<WorkspaceRecord> {
    if let Some(record) = state.workspace.get(workspace_id).await {
        return Ok(record);
    }

    let repo = state
        .store
        .repo_by_id(&row.repo_id)?
        .ok_or_else(|| Error::Git(format!("unknown repo {}", row.repo_id)))?;
    let repo_path = expand_tilde(&repo.path);
    let base = repo.default_branch_or_main().to_string();
    let status = WorkspaceLifecycle::parse(&row.status);

    Ok(WorkspaceRecord {
        id: row.id.clone(),
        repo_id: row.repo_id.clone(),
        repo_path: repo_path.to_string_lossy().to_string(),
        name: row.name.clone(),
        branch: row.branch.clone(),
        base,
        worktree_path: row.worktree_path.clone(),
        status,
        version: 0,
        activity: String::new(),
        prov_step: 0,
        setup_failed_command: None,
        setup_failed_exit_code: None,
    })
}

fn ensure_not_default_branch(state: &AppState, repo_id: &str, branch: &str) -> Result<()> {
    match state.store.repo_by_id(repo_id)? {
        Some(repo) => repo.ensure_not_default_branch(branch),
        None => Ok(()),
    }
}

pub async fn git_runtime_snapshot(
    state: &AppState,
    workspace_ids: &[String],
) -> Vec<WorkspaceGitRuntime> {
    let stats = state.workspace.git_stats_snapshot().await;
    workspace_ids
        .iter()
        .map(|id| {
            let git = stats.get(id).cloned().unwrap_or_default();
            WorkspaceGitRuntime {
                workspace_id: id.clone(),
                behind: git.behind,
                ahead: git.ahead,
                conflict: git.conflict,
            }
        })
        .collect()
}

pub async fn apply_behind_updates(state: &AppState, updates: &[crate::git::BehindUpdate]) {
    for update in updates {
        state
            .workspace
            .patch_git_stats(&update.workspace_id, |stats| {
                stats.behind = update.behind;
            })
            .await;
    }
}

pub async fn switch_workspace_branch(
    out: &Output,
    state: &AppState,
    workspace_id: &str,
    branch: &str,
) -> Result<()> {
    crate::git::validate_ref_name(branch, "branch")?;
    let row = state
        .store
        .workspace_by_id(workspace_id)?
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
    let record = resolve_record(state, workspace_id, &row).await?;
    state.workspace.remember(record.clone()).await;

    let previous = record.branch.clone();
    if previous == branch {
        return Ok(());
    }

    state.workspace.can_switch_branch(workspace_id).await?;
    ensure_not_default_branch(state, &row.repo_id, branch)?;

    let worktree = PathBuf::from(&record.worktree_path);
    state.git.switch(&worktree, branch).await?;

    state
        .workspace
        .switch_branch_record(workspace_id, branch)
        .await?;
    state.store.set_workspace_branch(workspace_id, branch)?;
    if row.pr_number.is_some() {
        state.store.set_workspace_pr_number(workspace_id, None)?;
    }

    refresh_git_stats(state, workspace_id, &worktree, &record.base, branch).await;

    let _ = state
        .diffs
        .compute(workspace_id, &worktree)
        .await
        .map_err(|error| log::warn!("diff refresh after branch switch: {error}"));

    if let Ok(thread_id) = lead_thread_id(state, workspace_id) {
        let _ = record_step(
            &state.store,
            &thread_id,
            TimelineStep::BranchSwitched {
                branch: branch.to_string(),
                change: BranchChange::CheckedOut {
                    previous: previous.to_string(),
                },
            },
        );
    }

    emit_switch_toast(out, &record.name, branch, workspace_id);

    emit_workspace_refresh(out, state);
    Ok(())
}

pub async fn pull_workspace(out: &Output, state: &AppState, workspace_id: &str) -> Result<()> {
    let row = state
        .store
        .workspace_by_id(workspace_id)?
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
    let record = resolve_record(state, workspace_id, &row).await?;
    state.workspace.remember(record.clone()).await;

    let behind = state.workspace.git_stats(workspace_id).await.behind;
    if behind == 0 {
        return Err(Error::Git(format!(
            "already up to date with {}",
            record.base
        )));
    }

    let worktree = PathBuf::from(&record.worktree_path);
    match state.git.merge(&worktree, &record.base).await {
        Ok(()) => {
            state.workspace.clear_git_conflict(workspace_id).await;
            refresh_git_stats(state, workspace_id, &worktree, &record.base, &record.branch).await;
            let _ = state.diffs.compute(workspace_id, &worktree).await;

            if let Ok(thread_id) = lead_thread_id(state, workspace_id) {
                let _ = record_step(
                    &state.store,
                    &thread_id,
                    TimelineStep::CommitsPulled {
                        count: behind as u32,
                        base: record.base.clone(),
                        branch: record.branch.clone(),
                    },
                );
            }

            emit_pull_toast(out, behind, &record.base, workspace_id);
            emit_git_state(out, state);
            Ok(())
        }
        Err(Error::GitConflict { operation, paths }) => {
            state
                .workspace
                .set_git_conflict(
                    workspace_id,
                    GitConflictState {
                        operation: parse_conflict_op(&operation),
                        paths,
                    },
                )
                .await;
            emit_git_state(out, state);
            Err(Error::Git(
                "merge stopped with conflicts — resolve or abort from the workspace header".into(),
            ))
        }
        Err(error) => Err(error),
    }
}

pub async fn rebase_workspace(out: &Output, state: &AppState, workspace_id: &str) -> Result<()> {
    let row = state
        .store
        .workspace_by_id(workspace_id)?
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
    let record = resolve_record(state, workspace_id, &row).await?;
    state.workspace.remember(record.clone()).await;

    let behind = state.workspace.git_stats(workspace_id).await.behind;
    if behind == 0 {
        return Err(Error::Git(format!("already on the latest {}", record.base)));
    }

    let repo_path = PathBuf::from(&record.repo_path);
    let worktree = PathBuf::from(&record.worktree_path);
    let pushed = state
        .git
        .remote_branch_exists(&repo_path, &record.branch)
        .await
        .unwrap_or(false);
    if pushed || row.pr_number.is_some() {
        log::info!(
            "rebase {}: branch was pushed or has an open PR; history rewrite may need force push",
            record.branch
        );
    }

    match state.git.rebase(&worktree, &record.base).await {
        Ok(()) => {
            state.workspace.clear_git_conflict(workspace_id).await;
            refresh_git_stats(state, workspace_id, &worktree, &record.base, &record.branch).await;
            let _ = state.diffs.compute(workspace_id, &worktree).await;

            if let Ok(thread_id) = lead_thread_id(state, workspace_id) {
                let _ = record_step(
                    &state.store,
                    &thread_id,
                    TimelineStep::BranchRebased {
                        branch: record.branch.clone(),
                        base: record.base.clone(),
                        replayed_onto: behind as u32,
                    },
                );
            }

            emit_rebase_toast(out, &record.branch, &record.base, workspace_id);
            emit_git_state(out, state);
            Ok(())
        }
        Err(Error::GitConflict { operation, paths }) => {
            state
                .workspace
                .set_git_conflict(
                    workspace_id,
                    GitConflictState {
                        operation: parse_conflict_op(&operation),
                        paths,
                    },
                )
                .await;
            emit_git_state(out, state);
            Err(Error::Git(
                "rebase stopped with conflicts — resolve or abort from the workspace header".into(),
            ))
        }
        Err(error) => Err(error),
    }
}

pub async fn push_workspace_branch(
    out: &Output,
    state: &AppState,
    workspace_id: &str,
) -> Result<()> {
    let row = state
        .store
        .workspace_by_id(workspace_id)?
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
    let record = resolve_record(state, workspace_id, &row).await?;
    let worktree = PathBuf::from(&record.worktree_path);
    state.git.push(&worktree, &record.branch).await?;
    refresh_git_stats(state, workspace_id, &worktree, &record.base, &record.branch).await;
    out.toast(ToastRaisedPayload {
        tone: ToastTone::Ok,
        parts: vec![
            ToastPart::Text {
                value: "Pushed ".into(),
            },
            ToastPart::Code {
                value: record.branch.clone(),
            },
        ],
        workspace_id: Some(workspace_id.to_string()),
        thread_id: None,
    });
    emit_git_state(out, state);
    Ok(())
}

pub async fn abort_workspace_git_conflict(
    out: &Output,
    state: &AppState,
    workspace_id: &str,
) -> Result<()> {
    let row = state
        .store
        .workspace_by_id(workspace_id)?
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
    let record = resolve_record(state, workspace_id, &row).await?;
    let conflict = state
        .workspace
        .git_stats(workspace_id)
        .await
        .conflict
        .clone();
    let Some(conflict) = conflict else {
        return Ok(());
    };
    let worktree = PathBuf::from(&record.worktree_path);
    match conflict.operation {
        GitConflictOperation::Merge => state.git.abort_merge(&worktree).await?,
        GitConflictOperation::Rebase => state.git.abort_rebase(&worktree).await?,
    }
    state.workspace.clear_git_conflict(workspace_id).await;
    refresh_git_stats(state, workspace_id, &worktree, &record.base, &record.branch).await;
    let _ = state.diffs.compute(workspace_id, &worktree).await;
    emit_git_state(out, state);
    Ok(())
}

pub async fn create_workspace_branch(
    out: &Output,
    state: &AppState,
    workspace_id: &str,
    branch: &str,
) -> Result<()> {
    let name = branch.trim();
    if name.is_empty() {
        return Err(Error::Git("branch name is required".into()));
    }
    let row = state
        .store
        .workspace_by_id(workspace_id)?
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
    let record = resolve_record(state, workspace_id, &row).await?;
    state.workspace.remember(record.clone()).await;
    state.workspace.can_switch_branch(workspace_id).await?;
    ensure_not_default_branch(state, &row.repo_id, name)?;
    let worktree = PathBuf::from(&record.worktree_path);
    state.git.switch_new_branch(&worktree, name, "HEAD").await?;
    state
        .workspace
        .switch_branch_record(workspace_id, name)
        .await?;
    state.store.set_workspace_branch(workspace_id, name)?;
    if row.pr_number.is_some() {
        state.store.set_workspace_pr_number(workspace_id, None)?;
    }
    refresh_git_stats(state, workspace_id, &worktree, &record.base, name).await;
    let _ = state.diffs.compute(workspace_id, &worktree).await;
    if let Ok(thread_id) = lead_thread_id(state, workspace_id) {
        let _ = record_step(
            &state.store,
            &thread_id,
            TimelineStep::BranchSwitched {
                branch: name.to_string(),
                change: BranchChange::Created {
                    previous: record.branch.clone(),
                },
            },
        );
    }
    emit_switch_toast(out, &record.name, name, workspace_id);
    emit_workspace_refresh(out, state);
    Ok(())
}

/// Bring the workspace in line with whatever branch an outside tool (`gh stack`) left
/// checked out and refresh its git stats and diff. Returns the branch.
pub async fn adopt_checked_out_branch(
    out: &Output,
    state: &AppState,
    workspace_id: &str,
    change: BranchChange,
) -> Result<String> {
    let row = state
        .store
        .workspace_by_id(workspace_id)?
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
    let record = resolve_record(state, workspace_id, &row).await?;
    let worktree = PathBuf::from(&record.worktree_path);
    let branch = state.git.current_branch(&worktree).await?;
    let switched = !branch.is_empty() && branch != record.branch;
    if switched {
        state
            .workspace
            .switch_branch_record(workspace_id, &branch)
            .await?;
        state.store.set_workspace_branch(workspace_id, &branch)?;
        if row.pr_number.is_some() {
            state.store.set_workspace_pr_number(workspace_id, None)?;
        }
        if let Ok(thread_id) = lead_thread_id(state, workspace_id) {
            let _ = record_step(
                &state.store,
                &thread_id,
                TimelineStep::BranchSwitched {
                    branch: branch.clone(),
                    change,
                },
            );
        }
        emit_switch_toast(out, &record.name, &branch, workspace_id);
    }
    let current = if branch.is_empty() {
        &record.branch
    } else {
        &branch
    };
    refresh_git_stats(state, workspace_id, &worktree, &record.base, current).await;
    let _ = state.diffs.compute(workspace_id, &worktree).await;
    emit_workspace_refresh(out, state);
    emit_git_state(out, state);
    Ok(current.clone())
}

/// How often each worktree's checked-out branch is compared with the one on record.
const BRANCH_WATCH_INTERVAL: Duration = Duration::from_secs(3);

/// The diff watcher only runs for the workspace on screen, so a checkout from a terminal or by
/// an agent in any other worktree would go unnoticed. Reading `HEAD` costs no process.
pub fn spawn_branch_watch(out: Output, state: AppState) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(BRANCH_WATCH_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            if let Err(error) = adopt_outside_checkouts(&out, &state).await {
                log::warn!("branch watch failed: {error}");
            }
        }
    });
}

async fn adopt_outside_checkouts(out: &Output, state: &AppState) -> Result<()> {
    for row in state.store.live_workspaces()? {
        if !matches!(
            WorkspaceLifecycle::parse(&row.status),
            WorkspaceLifecycle::Ready
                | WorkspaceLifecycle::Running
                | WorkspaceLifecycle::Idle
                | WorkspaceLifecycle::Waiting
        ) {
            continue;
        }
        let worktree = Path::new(&row.worktree_path);
        if !worktree.exists() {
            continue;
        }
        // Empty while HEAD is detached (mid-rebase); wait until a branch is checked out.
        let branch = match crate::git::checked_out_branch(worktree) {
            Some(branch) => branch,
            None => match state.git.current_branch(worktree).await {
                Ok(branch) => branch,
                Err(_) => continue,
            },
        };
        if branch.is_empty() || branch == row.branch {
            continue;
        }
        let change = BranchChange::CheckedOutOutside {
            previous: row.branch.clone(),
        };
        adopt_checked_out_branch(out, state, &row.id, change).await?;
    }
    Ok(())
}

fn parse_conflict_op(raw: &str) -> GitConflictOperation {
    if raw == "rebase" {
        GitConflictOperation::Rebase
    } else {
        GitConflictOperation::Merge
    }
}

/// Recounts how far the worktree is behind its base and ahead of its own branch on origin.
pub async fn refresh_git_stats(
    state: &AppState,
    workspace_id: &str,
    worktree: &Path,
    base: &str,
    branch: &str,
) {
    let behind = state.git.behind_count(worktree, base).await.unwrap_or(0);
    let ahead = state
        .git
        .unpushed_commit_count(worktree, branch)
        .await
        .unwrap_or(0);
    state
        .workspace
        .patch_git_stats(workspace_id, |stats| {
            stats.behind = behind;
            stats.ahead = ahead;
        })
        .await;
}

fn emit_git_state(out: &Output, state: &AppState) {
    state.notify(out, StateChangeKind::BehindCounts);
}

fn emit_workspace_refresh(out: &Output, state: &AppState) {
    state.notify(out, StateChangeKind::WorkspaceStatus);
}

pub fn lead_thread_id(state: &AppState, workspace_id: &str) -> Result<String> {
    state
        .store
        .lead_thread_id(workspace_id)?
        .ok_or_else(|| Error::Workspace("no Lead thread for workspace".into()))
}

fn emit_switch_toast(out: &Output, workspace_name: &str, branch: &str, workspace_id: &str) {
    out.toast(ToastRaisedPayload {
        tone: ToastTone::Ok,
        parts: vec![
            ToastPart::Text {
                value: "Switched ".into(),
            },
            ToastPart::Code {
                value: workspace_name.to_string(),
            },
            ToastPart::Text {
                value: " to ".into(),
            },
            ToastPart::Code {
                value: branch.to_string(),
            },
        ],
        workspace_id: Some(workspace_id.to_string()),
        thread_id: None,
    });
}

fn emit_pull_toast(out: &Output, behind: u32, base: &str, workspace_id: &str) {
    out.toast(ToastRaisedPayload {
        tone: ToastTone::Ok,
        parts: vec![
            ToastPart::Text {
                value: "Pulled ".into(),
            },
            ToastPart::Code {
                value: behind.to_string(),
            },
            ToastPart::Text {
                value: format!(" commit{} from ", if behind == 1 { "" } else { "s" }),
            },
            ToastPart::Code {
                value: base.to_string(),
            },
        ],
        workspace_id: Some(workspace_id.to_string()),
        thread_id: None,
    });
}

fn emit_rebase_toast(out: &Output, branch: &str, base: &str, workspace_id: &str) {
    out.toast(ToastRaisedPayload {
        tone: ToastTone::Ok,
        parts: vec![
            ToastPart::Text {
                value: "Rebased ".into(),
            },
            ToastPart::Code {
                value: branch.to_string(),
            },
            ToastPart::Text {
                value: " onto ".into(),
            },
            ToastPart::Code {
                value: base.to_string(),
            },
        ],
        workspace_id: Some(workspace_id.to_string()),
        thread_id: None,
    });
}

pub async fn rename_workspace(
    out: &Output,
    state: &AppState,
    input: RenameWorkspaceInput,
) -> Result<()> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(Error::Workspace("Workspace name is required".into()));
    }
    state.store.set_workspace_name(&input.workspace_id, name)?;
    state.workspace.rename(&input.workspace_id, name).await?;
    state.notify(out, StateChangeKind::WorkspaceStatus);
    Ok(())
}

/// Force-refresh the live diff snapshot for one workspace (branch switch, pull, etc.).
pub async fn refresh_diff(state: &AppState, workspace_id: &str) -> Result<()> {
    let workspace = load_workspace(state, workspace_id).await?;
    if let Ok(Some(row)) = state.store.workspace_by_id(workspace_id)
        && row.kind.as_deref() == Some("review")
    {
        state.diffs.default_diff_target(
            workspace_id,
            crate::git::DiffTarget::head_against(workspace.base.clone()),
        );
    }
    state
        .diffs
        .compute(workspace_id, Path::new(&workspace.worktree_path))
        .await?;
    Ok(())
}

/// Point the Changes panel at uncommitted work (`None`) or a branch against its base.
pub async fn set_diff_target(
    state: &AppState,
    workspace_id: &str,
    target: Option<crate::git::DiffTarget>,
) -> Result<()> {
    let workspace = load_workspace(state, workspace_id).await?;
    state.diffs.set_diff_target(workspace_id, target);
    state
        .diffs
        .compute(workspace_id, Path::new(&workspace.worktree_path))
        .await?;
    Ok(())
}

/// Throw away uncommitted changes to `paths` (all of them when empty), then refresh Changes.
pub async fn discard_changes(state: &AppState, workspace_id: &str, paths: &[String]) -> Result<()> {
    let workspace = load_workspace(state, workspace_id).await?;
    let worktree = Path::new(&workspace.worktree_path);
    state.git.discard(worktree, paths).await?;
    state.diffs.compute(workspace_id, worktree).await?;
    Ok(())
}

/// Open the worktree in the terminal app picked in Settings (Terminal by default).
pub async fn open_terminal(state: &AppState, workspace_id: &str) -> Result<()> {
    let row = state
        .store
        .workspace_by_id(workspace_id)?
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
    let terminal = terminal_app(state.store.get_setting("terminalApp")?);
    let status = tokio::process::Command::new("open")
        .args(["-a", &terminal, &row.worktree_path])
        .status()
        .await
        .map_err(|error| Error::Process(format!("open {terminal}: {error}")))?;
    if !status.success() {
        return Err(Error::Process(format!(
            "{terminal} couldn't open {}",
            row.worktree_path
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
