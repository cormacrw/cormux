use std::path::{Path, PathBuf};

use tauri::AppHandle;

use crate::error::{Error, Result};
use crate::feedback::{emit_toast, emit_toast_parts};
use crate::ipc::commands::emit_workspace_status;
use crate::ipc::types::{StateChangeKind, ToastPart, ToastTone};
use crate::ipc::events::StateChanged;
use crate::state::AppState;
use crate::workspace::{WorkspaceLifecycle, WorkspaceRecord};
use tauri_specta::Event;

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
    let row = state
        .store
        .workspace_by_id(workspace_id)?
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
    if row.archived_at.is_some() {
        return Err(Error::Workspace(format!(
            "workspace {workspace_id} is archived"
        )));
    }

    let record = resolve_record(state, workspace_id, &row).await?;
    let repo_path = PathBuf::from(&record.repo_path);
    let worktree = PathBuf::from(&record.worktree_path);
    let repo_name = repo_dir_name(&repo_path);

    let engine_label = lead_engine_label(state, workspace_id);
    let uncommitted_files = count_uncommitted_files(state, &worktree, row.modified_files).await;
    let unpushed_commits = if worktree.is_dir() {
        state
            .git
            .unpushed_commit_count(&worktree, &record.branch)
            .await
            .unwrap_or(0)
    } else {
        0
    };
    let remote_exists = state
        .git
        .remote_branch_exists(&repo_path, &record.branch)
        .await
        .unwrap_or(false);
    let has_data_loss = uncommitted_files > 0 || (!remote_exists && unpushed_commits > 0);
    let warning = data_loss_warning(uncommitted_files, unpushed_commits, remote_exists);

    let branch = record.branch.clone();
    Ok(TeardownPreview {
        workspace_id: workspace_id.to_string(),
        workspace_name: row.name,
        engine_label,
        branch: branch.clone(),
        worktree_path: display_worktree_path(repo_name, &branch),
        app_running: state.process.workspace_has_session(workspace_id),
        delete_branch_default: row.pr_number.is_none(),
        data_loss: TeardownDataLoss {
            uncommitted_files,
            unpushed_commits,
            has_data_loss,
            warning,
        },
    })
}

pub async fn execute(app: &AppHandle, state: &AppState, input: &TeardownInput) -> Result<()> {
    let row = state
        .store
        .workspace_by_id(&input.workspace_id)?
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {}", input.workspace_id)))?;
    if row.archived_at.is_some() {
        return Err(Error::Workspace(format!(
            "workspace {} is archived",
            input.workspace_id
        )));
    }

    let record = resolve_record(state, &input.workspace_id, &row).await?;
    if record.status == WorkspaceLifecycle::TearingDown {
        return Err(Error::Workspace(format!(
            "workspace {} is already being torn down",
            input.workspace_id
        )));
    }
    state.workspace.remember(record.clone()).await;
    let previous_status = record.status;
    let _ = state
        .workspace
        .set_status(&input.workspace_id, WorkspaceLifecycle::TearingDown)
        .await;
    emit_workspace_status(
        app,
        state,
        &input.workspace_id,
        WorkspaceLifecycle::TearingDown,
    );

    let thread_ids: Vec<String> = state
        .store
        .snapshot()?
        .threads
        .iter()
        .filter(|thread| thread.workspace_id == input.workspace_id)
        .map(|thread| thread.id.clone())
        .collect();

    if let Err(error) = run_teardown_steps(state, &record, input.delete_branch, &thread_ids).await
    {
        let _ = state
            .workspace
            .set_status(&input.workspace_id, previous_status)
            .await;
        emit_workspace_status(
            app,
            state,
            &input.workspace_id,
            previous_status,
        );
        return Err(error);
    }

    state.store.archive_workspace(&input.workspace_id)?;
    state.diffs.unwatch(&input.workspace_id);
    state.fetch.remove_workspace(&input.workspace_id).await;
    state.workspace.remove(&input.workspace_id).await;
    state.workspace.sync_fetch_targets().await;

    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(app);

    emit_teardown_toast(app, &row.name, &record.branch, input.delete_branch);

    Ok(())
}

async fn run_teardown_steps(
    state: &AppState,
    record: &WorkspaceRecord,
    delete_branch: bool,
    thread_ids: &[String],
) -> Result<()> {
    state.engines.stop_threads(thread_ids).await?;
    state.process.stop_workspace_sessions(&record.id)?;

    let repo = PathBuf::from(&record.repo_path);
    let worktree = PathBuf::from(&record.worktree_path);
    if worktree.exists() {
        let force = state
            .git
            .status_porcelain(&worktree)
            .await
            .map(|status| !status.trim().is_empty())
            .unwrap_or(false);
        state
            .git
            .worktree_remove(&repo, &worktree, force)
            .await?;
    }

    if delete_branch {
        state.git.branch_delete(&repo, &record.branch).await?;
    }

    Ok(())
}

async fn resolve_record(
    state: &AppState,
    workspace_id: &str,
    row: &crate::store::types::WorkspaceRow,
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

async fn count_uncommitted_files(
    state: &AppState,
    worktree: &Path,
    fallback: i64,
) -> u32 {
    if !worktree.is_dir() {
        return fallback.max(0) as u32;
    }
    match state.git.numstat(worktree).await {
        Ok(files) => files.len() as u32,
        Err(_) => fallback.max(0) as u32,
    }
}

fn lead_engine_label(state: &AppState, workspace_id: &str) -> String {
    let Ok(snapshot) = state.store.snapshot() else {
        return "the agent".into();
    };
    let engine = snapshot
        .threads
        .iter()
        .find(|thread| thread.workspace_id == workspace_id)
        .map(|thread| thread.engine.as_str())
        .unwrap_or("claude");
    engine_display(engine)
}

fn engine_display(engine: &str) -> String {
    match engine.to_lowercase().as_str() {
        "cursor" => "Cursor CLI".into(),
        "codex" => "Codex CLI".into(),
        "gemini" => "Gemini CLI".into(),
        _ => "Claude Code".into(),
    }
}

fn data_loss_warning(
    uncommitted_files: u32,
    unpushed_commits: u32,
    remote_exists: bool,
) -> Option<String> {
    let mut parts = Vec::new();
    if uncommitted_files > 0 {
        let label = if uncommitted_files == 1 {
            "1 uncommitted file change".into()
        } else {
            format!("{uncommitted_files} uncommitted file changes")
        };
        parts.push(format!("{label} will be discarded."));
    }
    if unpushed_commits > 0 && !remote_exists {
        let label = if unpushed_commits == 1 {
            "1 unpushed commit".into()
        } else {
            format!("{unpushed_commits} unpushed commits")
        };
        parts.push(format!("{label} will be lost."));
    }
    if parts.is_empty() {
        return None;
    }
    Some(format!("{} This can't be undone.", parts.join(" ")))
}

fn display_worktree_path(repo_name: &str, branch: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "~".into());
    format!(
        "{}/.harness/worktrees/{}/{}",
        home,
        repo_name,
        branch.replace('/', "-")
    )
}

fn repo_dir_name(repo_path: &Path) -> &str {
    repo_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo")
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

fn emit_teardown_toast(app: &AppHandle, name: &str, branch: &str, deleted_branch: bool) {
    if deleted_branch {
        emit_toast_parts(
            app,
            ToastTone::Bad,
            vec![
                ToastPart::Text {
                    value: format!("Tore down {name} and deleted local branch "),
                },
                ToastPart::Code {
                    value: branch.to_string(),
                },
            ],
            None,
        );
        return;
    }
    emit_toast(
        app,
        crate::ipc::types::ToastRaisedPayload {
            tone: ToastTone::Bad,
            parts: vec![ToastPart::Text {
                value: format!("Tore down {name}"),
            }],
            workspace_id: None,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_loss_message_covers_uncommitted_and_unpushed() {
        let message = data_loss_warning(2, 3, false).unwrap();
        assert!(message.contains("2 uncommitted file changes"));
        assert!(message.contains("3 unpushed commits"));
    }

    #[test]
    fn no_warning_when_remote_has_branch_and_clean() {
        assert!(data_loss_warning(0, 2, true).is_none());
    }
}
