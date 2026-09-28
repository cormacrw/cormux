use std::path::PathBuf;
use std::time::Duration;

use tauri::{AppHandle, State, ipc::Channel};
use tauri_specta::Event;
use uuid::Uuid;

use crate::composer::{persist_control_step, persist_user_message};
use crate::error::{Error, Result};
use crate::feedback::{emit_approval_counts, emit_toast, toast_for_approval};
use crate::workspace::ThreadActivity;
use crate::github::{auth, clear_token, save_token};
use crate::ipc::events::{StateChanged, WorkspaceStatusChanged};
use crate::ipc::types::StateChangeKind;
use crate::state::AppState;
use crate::store::types::{ThreadRow, WorkspaceRow};

use super::types::{
    AgentChunk, AgentEvent, ControlWorkspaceAppInput, CreateWorkspaceBranchInput,
    CreateWorkspaceInput, CreateWorkspaceResult, DiffUpdate, PtyChunk, RenameWorkspaceInput,
    RepoBranchesResult, ResolveApprovalResult, SetRepoRunCommandInput, Snapshot,
    SwitchWorkspaceBranchInput, TeardownInput, TeardownPreview, WorkspaceAppControlAction,
    WorkspaceSummaryResult,
};
use crate::app::WorkspaceAppAction;
use crate::store::types::RepoRecord;

#[tauri::command]
#[specta::specta]
pub async fn get_snapshot(state: State<'_, AppState>) -> Result<Snapshot> {
    let trees = process_trees(&state);
    let memory = state.metrics.sample(&trees).ok();
    let github_auth_configured = auth::resolve_token(&state.shell_env)
        .await
        .is_some();
    let pr_synced_at = state.store.get_setting("githubPrSyncedAt")?.filter(|value| !value.is_empty());

    let persisted = state.store.snapshot()?;
    let workspace_ids: Vec<String> = if state.workspace.list().await.is_empty() {
        persisted
            .workspaces
            .iter()
            .filter(|row| row.archived_at.is_none())
            .map(|row| row.id.clone())
            .collect()
    } else {
        state
            .workspace
            .list()
            .await
            .into_iter()
            .map(|row| row.id)
            .collect()
    };
    let workspace_git = crate::git_workspace::git_runtime_snapshot(&state, &workspace_ids);

    Ok(Snapshot {
        version: state.snapshot_version(),
        view: super::types::AppView::Homebase,
        persisted,
        workspaces: state.workspace.list().await,
        workspace_git,
        memory,
        pending_live_approvals: state.approvals.pending_count(),
        github_auth_configured,
        pr_synced_at,
        workspace_apps: state.apps.snapshot(),
    })
}

#[tauri::command]
#[specta::specta]
pub async fn control_workspace_app(
    app: AppHandle,
    input: ControlWorkspaceAppInput,
    state: State<'_, AppState>,
) -> Result<()> {
    let action = match input.action {
        WorkspaceAppControlAction::Run => WorkspaceAppAction::Run,
        WorkspaceAppControlAction::Restart => WorkspaceAppAction::Restart,
        WorkspaceAppControlAction::Stop => WorkspaceAppAction::Stop,
        WorkspaceAppControlAction::Clear => WorkspaceAppAction::Clear,
    };
    state
        .apps
        .control(&app, &state, &input.workspace_id, action)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn set_repo_run_command(
    app: AppHandle,
    input: SetRepoRunCommandInput,
    state: State<'_, AppState>,
) -> Result<()> {
    let repos = state.store.snapshot()?.repos;
    let Some(mut repo) = repos.into_iter().find(|row| row.id == input.repo_id) else {
        return Err(Error::Workspace(format!("unknown repo {}", input.repo_id)));
    };
    repo.run_command = input
        .run_command
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    state.store.upsert_repo(&RepoRecord {
        id: repo.id,
        path: repo.path,
        name: repo.name,
        default_branch: repo.default_branch,
        setup_commands: repo.setup_commands,
        run_command: repo.run_command,
    })?;
    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(&app);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn reload_environment(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    state.shell_env.write().await.reload().await?;
    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::Environment,
    }
    .emit(&app);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn fetch_on_focus(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    let updates = state.fetch.tick().await?;
    crate::git_workspace::apply_behind_updates(&state, &updates).await;
    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::BehindCounts,
    }
    .emit(&app);
    if let Err(error) = state.pr_sync.sync_app(&app).await {
        log::warn!("focus PR sync failed: {error}");
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn get_metrics(state: State<'_, AppState>) -> Result<crate::metrics::MemorySample> {
    state.metrics.sample(&process_trees(&state))
}

#[tauri::command]
#[specta::specta]
pub async fn set_github_token(token: String) -> Result<()> {
    save_token(&token)
}

#[tauri::command]
#[specta::specta]
pub async fn clear_github_token() -> Result<()> {
    clear_token()
}

#[tauri::command]
#[specta::specta]
pub async fn sync_pull_requests(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    state.pr_sync.sync_app(&app).await
}

/// High-volume ordered stream of agent message chunks for one thread.
#[tauri::command]
#[specta::specta]
pub fn subscribe_agent_chunks(
    thread_id: String,
    channel: Channel<AgentChunk>,
    state: State<'_, AppState>,
) -> Result<()> {
    let engines = state.engines.clone();
    tauri::async_runtime::spawn(async move {
        let rx = loop {
            match engines.subscribe(&thread_id) {
                Ok(rx) => break rx,
                Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
            }
        };
        let mut rx = rx;
        while let Ok(event) = rx.recv().await {
            if let crate::engines::AgentEvent::MessageChunk { text, .. } = event {
                let _ = channel.send(AgentChunk {
                    thread_id: thread_id.clone(),
                    text,
                });
            }
        }
    });
    Ok(())
}

/// High-volume ordered stream of normalised agent events for one thread.
#[tauri::command]
#[specta::specta]
pub fn subscribe_agent_events(
    thread_id: String,
    channel: Channel<AgentEvent>,
    state: State<'_, AppState>,
) -> Result<()> {
    let engines = state.engines.clone();
    tauri::async_runtime::spawn(async move {
        let rx = loop {
            match engines.subscribe(&thread_id) {
                Ok(rx) => break rx,
                Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
            }
        };
        let mut rx = rx;
        while let Ok(event) = rx.recv().await {
            let _ = channel.send(event);
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn detect_engines(
    state: State<'_, AppState>,
) -> Result<Vec<crate::engines::EngineStatus>> {
    state.engines.detect().await
}

#[tauri::command]
#[specta::specta]
pub async fn resolve_approval(
    app: AppHandle,
    id: String,
    approved: bool,
    deny_reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<ResolveApprovalResult> {
    let (row, payload) =
        crate::approvals::mark_resolved(&state.store, &id, approved, deny_reason.clone())?;
    let deny_message = if approved {
        None
    } else {
        deny_reason.or(payload.deny_reason.clone())
    };
    state
        .approvals
        .resolve(&id, approved, deny_message.clone())
        .await?;
    let hints = crate::approvals::apply_harness_effects(&state, &row, &payload, approved)?;
    if !approved {
        if let Some(reason) = deny_message.filter(|value| !value.trim().is_empty()) {
            let _ = state
                .engines
                .prompt(&row.thread_id, format!("Approval denied: {reason}"));
        }
    }
    emit_approval_counts(&app, &state);
    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(&app);
    if let Some(payload) = toast_for_approval(&row.tool, approved) {
        emit_toast(&app, payload);
    }
    Ok(ResolveApprovalResult {
        focus_composer: hints.focus_composer,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn resolve_all_approvals(
    app: AppHandle,
    thread_id: String,
    approved: bool,
    deny_reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<ResolveApprovalResult> {
    let pending = state.store.pending_approvals_for_thread(&thread_id)?;
    let mut focus_composer = false;
    for row in pending {
        let result = resolve_approval(
            app.clone(),
            row.id,
            approved,
            deny_reason.clone(),
            state.clone(),
        )
        .await?;
        focus_composer |= result.focus_composer;
    }
    Ok(ResolveApprovalResult { focus_composer })
}

/// High-volume ordered stream of PTY output for a workspace's run/setup log.
#[tauri::command]
#[specta::specta]
pub fn subscribe_pty(
    workspace_id: String,
    channel: Channel<PtyChunk>,
    state: State<'_, AppState>,
) -> Result<()> {
    let process = state.process.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let session_id = format!("{workspace_id}-app");
            let mut lines = process.drain_pending(&workspace_id);
            lines.extend(process.drain_pending(&session_id));
            for line in lines {
                let _ = channel.send(PtyChunk {
                    workspace_id: workspace_id.clone(),
                    line,
                });
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    });
    Ok(())
}

/// High-volume ordered stream of live diff updates for a worktree.
#[tauri::command]
#[specta::specta]
pub fn subscribe_diffs(
    workspace_id: String,
    channel: Channel<DiffUpdate>,
    state: State<'_, AppState>,
) -> Result<()> {
    let diffs = state.diffs.clone();
    tauri::async_runtime::spawn(async move {
        let mut last = None;
        loop {
            if let Some(diff) = diffs.latest(&workspace_id) {
                if last.as_ref() != Some(&diff) {
                    let _ = channel.send(DiffUpdate {
                        workspace_id: workspace_id.clone(),
                        path: String::new(),
                        diff: Some(diff.clone()),
                    });
                    last = Some(diff);
                }
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    });
    Ok(())
}

/// Force-refresh the live diff snapshot for one workspace (branch switch, pull, etc.).
#[tauri::command]
#[specta::specta]
pub async fn refresh_workspace_diff(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<()> {
    use std::path::Path;

    let workspace = state
        .workspace
        .get(&workspace_id)
        .await
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
    state
        .diffs
        .compute(&workspace_id, Path::new(&workspace.worktree_path))
        .await?;
    Ok(())
}

/// Approve stages the file; reject discards worktree edits for that path.
#[tauri::command]
#[specta::specta]
pub async fn review_worktree_file(
    workspace_id: String,
    path: String,
    decision: String,
    state: State<'_, AppState>,
) -> Result<()> {
    use std::path::Path;

    let workspace = state
        .workspace
        .get(&workspace_id)
        .await
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
    let worktree = Path::new(&workspace.worktree_path);
    match decision.as_str() {
        "approve" => state.git.stage_worktree_file(worktree, &path).await?,
        "reject" => state.git.discard_worktree_file(worktree, &path).await?,
        other => {
            return Err(Error::Workspace(format!(
                "unknown review decision {other}"
            )))
        }
    }
    state.diffs.compute(&workspace_id, worktree).await?;
    Ok(())
}

/// Generate or return a cached workspace card summary (debounced LLM + local fallback).
#[tauri::command]
#[specta::specta]
pub async fn summarise_workspace(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<WorkspaceSummaryResult> {
    use crate::error::Error;
    use crate::summaries::{self, SUMMARY_MODEL_LABEL};

    let snapshot = state.store.snapshot()?;
    let workspace = snapshot
        .workspaces
        .iter()
        .find(|row| row.id == workspace_id)
        .cloned()
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;

    if let (Some(summary), Some(at)) = (&workspace.summary, &workspace.summary_at) {
        if !summary.is_empty() {
            return Ok(WorkspaceSummaryResult {
                workspace_id,
                summary: summary.clone(),
                summary_at: at.clone(),
                summary_source: workspace.summary_source.clone(),
                from_llm: true,
            });
        }
    }

    let threads: Vec<_> = snapshot
        .threads
        .iter()
        .filter(|thread| thread.workspace_id == workspace_id)
        .cloned()
        .collect();

    let prompt = summaries::build_summary_prompt(&workspace, &threads);
    let (summary, from_llm) = match state.llm.summarise(&workspace_id, &prompt).await {
        Ok(Some(result)) if !result.text.is_empty() => (result.text, true),
        _ => (
            summaries::fallback_summary(&workspace, &threads),
            false,
        ),
    };

    state
        .store
        .set_workspace_summary(&workspace_id, &summary, SUMMARY_MODEL_LABEL)?;

    let updated = state
        .store
        .workspace_by_id(&workspace_id)?
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;

    Ok(WorkspaceSummaryResult {
        workspace_id,
        summary,
        summary_at: updated
            .summary_at
            .unwrap_or_else(|| "now".into()),
        summary_source: updated.summary_source,
        from_llm,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn list_repo_branches(
    repo_id: String,
    state: State<'_, AppState>,
) -> Result<RepoBranchesResult> {
    let snapshot = state.store.snapshot()?;
    let repo = snapshot
        .repos
        .into_iter()
        .find(|row| row.id == repo_id)
        .ok_or_else(|| Error::Git(format!("unknown repo {repo_id}")))?;
    let repo_path = expand_tilde(&repo.path);
    let mut names = state.git.list_local_branches(&repo_path).await?;
    names.sort();
    names.dedup();
    Ok(RepoBranchesResult { branches: names })
}

#[tauri::command]
#[specta::specta]
pub async fn create_workspace(
    input: CreateWorkspaceInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<CreateWorkspaceResult> {
    let snapshot = state.store.snapshot()?;
    let repo = snapshot
        .repos
        .into_iter()
        .find(|row| row.id == input.repo_id)
        .ok_or_else(|| Error::Git(format!("unknown repo {}", input.repo_id)))?;
    let repo_path = expand_tilde(&repo.path);
    if !repo_path.is_dir() {
        return Err(Error::Git(format!(
            "repo path does not exist: {}",
            repo_path.display()
        )));
    }

    let workspace_id = new_workspace_id();
    let thread_id = Uuid::new_v4().to_string();
    let repo_name = repo_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo");
    let worktree_path = crate::workspace::WorkspaceManager::worktree_path(
        &harness_home(),
        repo_name,
        &input.branch,
    );

    state
        .workspace
        .register_provisioning(
            &workspace_id,
            &input.repo_id,
            &repo_path,
            &input.name,
            &input.branch,
            &input.base,
            &harness_home(),
        )
        .await?;

    state.store.upsert_workspace(&WorkspaceRow {
        id: workspace_id.clone(),
        repo_id: input.repo_id.clone(),
        name: input.name.clone(),
        branch: input.branch.clone(),
        worktree_path: worktree_path.to_string_lossy().to_string(),
        status: "provisioning".into(),
        created_at: String::new(),
        summary: None,
        summary_at: None,
        summary_source: "Haiku 4.5".into(),
        kind: None,
        pr_number: None,
        modified_files: 0,
        archived_at: None,
    })?;
    state.store.upsert_thread(&ThreadRow {
        id: thread_id.clone(),
        workspace_id: workspace_id.clone(),
        title: "Lead".into(),
        engine: input.engine.clone(),
        session_id: None,
        status: "provisioning".into(),
        used_tokens: None,
        context_size: None,
        cost_usd: None,
        transcript_readonly: false,
    })?;
    let message = serde_json::json!({ "role": "user", "text": input.goal });
    state
        .store
        .append_event(&thread_id, "message", &message.to_string())?;
    let worktree_step = serde_json::json!({
        "icon": "branch",
        "title": "Created worktree",
        "detail": format!("{} ← {}", input.branch, input.base),
    });
    state
        .store
        .append_event(&thread_id, "tool", &worktree_step.to_string())?;

    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(&app);

    let app_handle = app.clone();
    let name = input.name.clone();
    let repo_id_bg = input.repo_id.clone();
    let workspace_id_bg = workspace_id.clone();
    let thread_id_bg = thread_id.clone();
    let engine = input.engine.clone();
    let goal = input.goal.clone();
    let repo_name = repo.name.clone();
    let setup_commands = repo.setup_commands.clone();

    emit_toast(
        &app,
        crate::ipc::types::ToastRaisedPayload {
            tone: crate::ipc::types::ToastTone::Ok,
            parts: vec![
                crate::ipc::types::ToastPart::Text {
                    value: format!("Created {name} on "),
                },
                crate::ipc::types::ToastPart::Code {
                    value: input.branch.clone(),
                },
                crate::ipc::types::ToastPart::Text {
                    value: " from ".into(),
                },
                crate::ipc::types::ToastPart::Code {
                    value: input.base.clone(),
                },
            ],
            workspace_id: Some(workspace_id.clone()),
        },
    );

    tauri::async_runtime::spawn(async move {
        crate::provisioning::run_workspace_provisioning(
            app_handle,
            crate::provisioning::LeadProvisionJob {
                workspace_id: workspace_id_bg,
                thread_id: thread_id_bg,
                repo_id: repo_id_bg,
                repo_name,
                setup_commands_raw: setup_commands,
                engine,
                goal,
                review: false,
            },
        )
        .await;
    });

    Ok(CreateWorkspaceResult { workspace_id })
}

#[tauri::command]
#[specta::specta]
pub async fn retry_workspace_provisioning(
    workspace_id: String,
    app: AppHandle,
) -> Result<()> {
    tauri::async_runtime::spawn(async move {
        crate::provisioning::retry_provisioning(app, workspace_id).await;
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn skip_workspace_provisioning_setup(
    workspace_id: String,
    app: AppHandle,
) -> Result<()> {
    tauri::async_runtime::spawn(async move {
        crate::provisioning::skip_provisioning_setup(app, workspace_id).await;
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn rename_workspace(
    app: AppHandle,
    state: State<'_, AppState>,
    input: RenameWorkspaceInput,
) -> Result<()> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(Error::Workspace("Workspace name is required".into()));
    }
    state
        .store
        .set_workspace_name(&input.workspace_id, name)?;
    state
        .workspace
        .rename(&input.workspace_id, name)
        .await?;
    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(&app);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn switch_workspace_branch(
    input: SwitchWorkspaceBranchInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::git_workspace::switch_workspace_branch(&app, &state, &input.workspace_id, &input.branch)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn pull_workspace(
    workspace_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::git_workspace::pull_workspace(&app, &state, &workspace_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn rebase_workspace(
    workspace_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::git_workspace::rebase_workspace(&app, &state, &workspace_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn push_workspace_branch(
    workspace_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::git_workspace::push_workspace_branch(&app, &state, &workspace_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn abort_workspace_git(
    workspace_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::git_workspace::abort_workspace_git_conflict(&app, &state, &workspace_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn create_workspace_branch(
    input: CreateWorkspaceBranchInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::git_workspace::create_workspace_branch(
        &app,
        &state,
        &input.workspace_id,
        &input.branch,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn get_teardown_preview(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<TeardownPreview> {
    crate::teardown::preview(&state, &workspace_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn teardown_workspace(
    input: TeardownInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::teardown::execute(&app, &state, &input).await
}

#[tauri::command]
#[specta::specta]
pub async fn join_workspace_thread(
    input: super::types::JoinWorkspaceThreadInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<super::types::JoinWorkspaceThreadResult> {
    let thread_id = Uuid::new_v4().to_string();
    let snapshot = state.store.snapshot()?;
    let workspace_row = snapshot
        .workspaces
        .iter()
        .find(|row| row.id == input.workspace_id)
        .cloned()
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {}", input.workspace_id)))?;
    let agent_count = snapshot
        .threads
        .iter()
        .filter(|row| row.workspace_id == input.workspace_id)
        .count()
        + 1;

    state.store.upsert_thread(&ThreadRow {
        id: thread_id.clone(),
        workspace_id: input.workspace_id.clone(),
        title: input.title.clone(),
        engine: input.engine.clone(),
        session_id: None,
        status: "provisioning".into(),
        used_tokens: None,
        context_size: None,
        cost_usd: None,
        transcript_readonly: false,
    })?;

    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(&app);

    let thread_id_return = thread_id.clone();
    let app_handle = app.clone();
    let workspace_id = input.workspace_id.clone();
    let engine = input.engine.clone();
    let branch = workspace_row.branch.clone();
    tauri::async_runtime::spawn(async move {
        crate::provisioning::join_thread_provisioning(
            app_handle,
            crate::provisioning::JoinProvisionJob {
                workspace_id,
                thread_id,
                engine,
                branch,
                agent_count,
            },
        )
        .await;
    });

    Ok(super::types::JoinWorkspaceThreadResult {
        thread_id: thread_id_return,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn send_thread_prompt(
    app: AppHandle,
    thread_id: String,
    text: String,
    state: State<'_, AppState>,
) -> Result<()> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Ok(());
    }
    let thread = snapshot_thread(&state, &thread_id)?;
    let held = thread.status == "paused";
    persist_user_message(&state.store, &thread_id, &text)?;
    let _ = state
        .engines
        .submit_prompt(&thread_id, text, held);
    emit_composer_snapshot(&app, &state);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn cancel_thread_turn(thread_id: String, state: State<'_, AppState>) -> Result<()> {
    let _ = state.engines.cancel(&thread_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn pause_thread(
    app: AppHandle,
    thread_id: String,
    state: State<'_, AppState>,
) -> Result<()> {
    let thread = snapshot_thread(&state, &thread_id)?;
    if thread.status != "running" {
        return Ok(());
    }
    state.engines.hold_thread(&thread_id)?;
    persist_control_step(&state.store, &thread_id, "You paused the agent")?;
    state.store.set_thread_status(&thread_id, "paused")?;
    state
        .workspace
        .set_thread(&thread_id, &thread.workspace_id, ThreadActivity::Paused)
        .await;
    emit_composer_snapshot(&app, &state);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn resume_thread(
    app: AppHandle,
    thread_id: String,
    state: State<'_, AppState>,
) -> Result<()> {
    let thread = snapshot_thread(&state, &thread_id)?;
    if thread.status != "paused" {
        return Ok(());
    }
    persist_control_step(&state.store, &thread_id, "You resumed the agent")?;
    state.store.set_thread_status(&thread_id, "running")?;
    state
        .workspace
        .set_thread(&thread_id, &thread.workspace_id, ThreadActivity::Running)
        .await;
    if let Some(text) = state.engines.release_thread(&thread_id)? {
        let _ = state.engines.prompt(&thread_id, text);
    }
    emit_composer_snapshot(&app, &state);
    Ok(())
}

/// Spike 5: stream agent chunks (~60hz) and PTY lines (100/s) for a few seconds.
#[tauri::command]
#[specta::specta]
pub fn start_streaming_spike(agent: Channel<AgentChunk>, pty: Channel<PtyChunk>) -> Result<()> {
    tauri::async_runtime::spawn(async move {
        let started = std::time::Instant::now();
        let mut n = 0u32;
        while started.elapsed() < std::time::Duration::from_secs(6) {
            n += 1;
            let _ = agent.send(AgentChunk {
                thread_id: "spike".into(),
                text: format!("token-{n} "),
            });
            let _ = pty.send(PtyChunk {
                workspace_id: "spike".into(),
                line: format!("[{n}] app log line at 100hz"),
            });
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    });
    Ok(())
}

pub fn emit_workspace_status(
    app: &AppHandle,
    state: &AppState,
    workspace_id: &str,
    status: crate::workspace::WorkspaceLifecycle,
) {
    let version = state.bump_event_version();
    let _ = WorkspaceStatusChanged {
        version,
        workspace_id: workspace_id.to_string(),
        status,
    }
    .emit(app);
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(app);
}

fn harness_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

fn new_workspace_id() -> String {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("ws{ms:x}")
}

fn snapshot_thread(state: &AppState, thread_id: &str) -> Result<ThreadRow> {
    state
        .store
        .snapshot()?
        .threads
        .into_iter()
        .find(|row| row.id == thread_id)
        .ok_or_else(|| Error::Store(format!("unknown thread {thread_id}")))
}

fn emit_composer_snapshot(app: &AppHandle, state: &AppState) {
    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(app);
}

fn process_trees(state: &AppState) -> Vec<(String, Vec<u32>)> {
    let mut trees: std::collections::HashMap<String, Vec<u32>> = std::collections::HashMap::new();
    for (id, pid) in state.process.pids() {
        trees.entry(id).or_default().push(pid);
    }
    trees.into_iter().collect()
}
