use std::path::{Path, PathBuf};
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::error::{Error, Result};
use crate::feedback::emit_toast;
use crate::ipc::events::StateChanged;
use crate::ipc::types::{
    GitConflictOperation, GitConflictState, StateChangeKind, ToastPart, ToastRaisedPayload,
    ToastTone, WorkspaceGitRuntime,
};
use crate::state::AppState;
use crate::store::Store;
use crate::store::types::WorkspaceRow;
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

pub async fn resolve_record(
    state: &AppState,
    workspace_id: &str,
    row: &WorkspaceRow,
) -> Result<WorkspaceRecord> {
    if let Some(record) = state.workspace.get(workspace_id).await {
        return Ok(record);
    }

    let snapshot = state.store.snapshot()?;
    let repo = snapshot
        .repos
        .iter()
        .find(|repo| repo.id == row.repo_id)
        .ok_or_else(|| Error::Git(format!("unknown repo {}", row.repo_id)))?;
    let repo_path = expand_tilde(&repo.path);
    let base = repo.default_branch.clone().unwrap_or_else(|| "main".into());
    let status = parse_lifecycle(&row.status);

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
    match state
        .store
        .snapshot()?
        .repos
        .iter()
        .find(|repo| repo.id == repo_id)
    {
        Some(repo) => repo.ensure_not_default_branch(branch),
        None => Ok(()),
    }
}

pub fn git_runtime_snapshot(
    state: &AppState,
    workspace_ids: &[String],
) -> Vec<WorkspaceGitRuntime> {
    let stats = state.workspace.git_stats_snapshot();
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
    app: &AppHandle,
    state: &AppState,
    workspace_id: &str,
    branch: &str,
) -> Result<()> {
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

    state.diffs.request_refresh(workspace_id);
    let _ = state
        .diffs
        .compute(workspace_id, &worktree)
        .await
        .map_err(|error| log::warn!("diff refresh after branch switch: {error}"));

    if let Ok(thread_id) = lead_thread_id(state, workspace_id) {
        let _ = append_git_step(
            &state.store,
            &thread_id,
            "branch",
            &format!("Switched to `{branch}`"),
            &format!("Checked out {branch} in this worktree, previously {previous}"),
        );
    }

    emit_switch_toast(app, &record.name, branch, workspace_id);

    emit_workspace_refresh(app, state);
    Ok(())
}

pub async fn pull_workspace(app: &AppHandle, state: &AppState, workspace_id: &str) -> Result<()> {
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
            state.diffs.request_refresh(workspace_id);
            let _ = state.diffs.compute(workspace_id, &worktree).await;

            if let Ok(thread_id) = lead_thread_id(state, workspace_id) {
                let _ = append_git_step(
                    &state.store,
                    &thread_id,
                    "download",
                    &format!(
                        "Pulled {behind} commit{} from {}",
                        if behind == 1 { "" } else { "s" },
                        record.base
                    ),
                    &format!("Merged into {} with no conflicts", record.branch),
                );
            }

            emit_pull_toast(app, &record.name, behind, &record.base, workspace_id);
            emit_git_state(app, state).await;
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
            emit_git_state(app, state).await;
            Err(Error::Git(
                "merge stopped with conflicts — resolve or abort from the workspace header".into(),
            ))
        }
        Err(error) => Err(error),
    }
}

pub async fn rebase_workspace(app: &AppHandle, state: &AppState, workspace_id: &str) -> Result<()> {
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
            state.diffs.request_refresh(workspace_id);
            let _ = state.diffs.compute(workspace_id, &worktree).await;

            if let Ok(thread_id) = lead_thread_id(state, workspace_id) {
                let _ = append_git_step(
                    &state.store,
                    &thread_id,
                    "branch",
                    &format!("Rebased {} onto {}", record.branch, record.base),
                    &format!(
                        "Replayed local commits on top of {behind} new commit{} with no conflicts",
                        if behind == 1 { "" } else { "s" }
                    ),
                );
            }

            emit_rebase_toast(app, &record.branch, &record.base, workspace_id);
            emit_git_state(app, state).await;
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
            emit_git_state(app, state).await;
            Err(Error::Git(
                "rebase stopped with conflicts — resolve or abort from the workspace header".into(),
            ))
        }
        Err(error) => Err(error),
    }
}

pub async fn push_workspace_branch(
    app: &AppHandle,
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
    emit_toast(
        app,
        ToastRaisedPayload {
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
        },
    );
    emit_git_state(app, state).await;
    Ok(())
}

pub async fn abort_workspace_git_conflict(
    app: &AppHandle,
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
    state.diffs.request_refresh(workspace_id);
    let _ = state.diffs.compute(workspace_id, &worktree).await;
    emit_git_state(app, state).await;
    Ok(())
}

pub async fn create_workspace_branch(
    app: &AppHandle,
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
    state.diffs.request_refresh(workspace_id);
    let _ = state.diffs.compute(workspace_id, &worktree).await;
    if let Ok(thread_id) = lead_thread_id(state, workspace_id) {
        let _ = append_git_step(
            &state.store,
            &thread_id,
            "branch",
            &format!("Switched to `{name}`"),
            &format!(
                "Created and checked out {name}, previously {}",
                record.branch
            ),
        );
    }
    emit_switch_toast(app, &record.name, name, workspace_id);
    emit_workspace_refresh(app, state);
    Ok(())
}

/// Bring the workspace in line with whatever branch an outside tool (`gh stack`) left
/// checked out and refresh its git stats and diff. Returns the branch.
pub async fn adopt_checked_out_branch(
    app: &AppHandle,
    state: &AppState,
    workspace_id: &str,
    detail: &str,
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
            let _ = append_git_step(
                &state.store,
                &thread_id,
                "branch",
                &format!("Switched to `{branch}`"),
                detail,
            );
        }
        emit_switch_toast(app, &record.name, &branch, workspace_id);
    }
    let current = if branch.is_empty() {
        &record.branch
    } else {
        &branch
    };
    refresh_git_stats(state, workspace_id, &worktree, &record.base, current).await;
    state.diffs.request_refresh(workspace_id);
    let _ = state.diffs.compute(workspace_id, &worktree).await;
    emit_workspace_refresh(app, state);
    emit_git_state(app, state).await;
    Ok(current.clone())
}

/// How often each worktree's checked-out branch is compared with the one on record.
const BRANCH_WATCH_INTERVAL: Duration = Duration::from_secs(3);

/// A worktree's HEAD lives under the main repo's `.git/worktrees/`, outside the folder the
/// diff watcher sees, so a checkout from a terminal or by the agent would go unnoticed.
pub fn spawn_branch_watch(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(BRANCH_WATCH_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            let state = app.state::<AppState>();
            if let Err(error) = adopt_outside_checkouts(&app, &state).await {
                log::warn!("branch watch failed: {error}");
            }
        }
    });
}

async fn adopt_outside_checkouts(app: &AppHandle, state: &AppState) -> Result<()> {
    for row in state.store.live_workspaces()? {
        if !matches!(
            parse_lifecycle(&row.status),
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
        let Ok(branch) = state.git.current_branch(worktree).await else {
            continue;
        };
        if branch.is_empty() || branch == row.branch {
            continue;
        }
        let detail = format!(
            "Checked out {branch} outside Cormux, previously {}",
            row.branch
        );
        adopt_checked_out_branch(app, state, &row.id, &detail).await?;
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

async fn refresh_git_stats(
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

async fn emit_git_state(app: &AppHandle, state: &AppState) {
    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::BehindCounts,
    }
    .emit(app);
}

fn emit_workspace_refresh(app: &AppHandle, state: &AppState) {
    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(app);
}

pub fn lead_thread_id(state: &AppState, workspace_id: &str) -> Result<String> {
    state
        .store
        .snapshot()?
        .threads
        .iter()
        .find(|thread| thread.workspace_id == workspace_id && thread.title == "Lead")
        .map(|thread| thread.id.clone())
        .ok_or_else(|| Error::Workspace("no Lead thread for workspace".into()))
}

fn append_git_step(
    store: &Store,
    thread_id: &str,
    icon: &str,
    title: &str,
    detail: &str,
) -> Result<()> {
    let step = serde_json::json!({
        "icon": icon,
        "title": title,
        "detail": detail,
    });
    let _ = store.append_event(thread_id, "tool", &step.to_string())?;
    Ok(())
}

fn emit_switch_toast(app: &AppHandle, workspace_name: &str, branch: &str, workspace_id: &str) {
    emit_toast(
        app,
        ToastRaisedPayload {
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
        },
    );
}

fn emit_pull_toast(
    app: &AppHandle,
    _workspace_name: &str,
    behind: u32,
    base: &str,
    workspace_id: &str,
) {
    emit_toast(
        app,
        ToastRaisedPayload {
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
        },
    );
}

fn emit_rebase_toast(app: &AppHandle, branch: &str, base: &str, workspace_id: &str) {
    emit_toast(
        app,
        ToastRaisedPayload {
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
        },
    );
}

fn parse_lifecycle(raw: &str) -> WorkspaceLifecycle {
    match raw {
        "creating" => WorkspaceLifecycle::Creating,
        "provisioning" => WorkspaceLifecycle::Provisioning,
        "running" => WorkspaceLifecycle::Running,
        "idle" => WorkspaceLifecycle::Idle,
        "waiting" => WorkspaceLifecycle::Waiting,
        "tearingDown" => WorkspaceLifecycle::TearingDown,
        "gone" => WorkspaceLifecycle::Gone,
        "provisioningFailed" => WorkspaceLifecycle::ProvisioningFailed,
        _ => WorkspaceLifecycle::Ready,
    }
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(stripped);
    }
    PathBuf::from(path)
}
