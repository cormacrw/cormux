use std::path::{Path, PathBuf};
use std::time::Duration;

use tauri::{AppHandle, State, ipc::Channel};
use tauri_specta::Event;
use uuid::Uuid;

use crate::clickup::types::{ClickupBoard, ClickupOption, ClickupTaskDetail};
use crate::composer::{persist_control_step, persist_user_message};
use crate::engines::{EngineKind, ThreadModels};
use crate::error::{Error, Result};
use crate::feedback::{emit_approval_counts, emit_toast, toast_for_approval};
use crate::ipc::events::{StateChanged, WorkspaceStatusChanged};
use crate::ipc::subscriptions::SubscriptionHandle;
use crate::ipc::types::StateChangeKind;
use crate::state::AppState;
use crate::store::types::{ThreadRow, TodoRow, WorkspaceRow};
use crate::workspace::ThreadActivity;

use super::types::{
    AddRepoInput, AgentChunk, AgentEvent, BranchDeletion, ControlWorkspaceAppInput,
    CreateWorkspaceBranchInput, CreateWorkspaceInput, CreateWorkspacePullRequestInput,
    CreateWorkspacePullRequestResult, CreateWorkspaceResult, DiffUpdate, DraftPrWhyResult,
    LocalBranchRow, LocalBranchesResult, PtyChunk, RemoveRepoInput, RenameWorkspaceInput,
    RepoBranchesResult, RepoGitRuntime, ResolveApprovalResult, SendWorkspaceFindingsInput,
    SetRepoDefaultBranchInput, SetRepoRunCommandInput, SetRepoSetupCommandsInput,
    SetRepoSingleInstanceInput, SetSettingInput, Snapshot, SwitchWorkspaceBranchInput,
    TeardownInput, TeardownPreview, TestRepoSetupInput, TestRepoSetupResult,
    WorkspaceAppControlAction, WorkspaceSummaryResult,
};
use crate::app::WorkspaceAppAction;
use crate::store::types::RepoRecord;

/// Settings key for the ClickUp folder whose lists are the sprints.
const CLICKUP_FOLDER_KEY: &str = "clickupFolderId";

#[tauri::command]
#[specta::specta]
pub async fn get_snapshot(state: State<'_, AppState>) -> Result<Snapshot> {
    let trees = process_trees(&state);
    let memory = state.metrics.sample(&trees).ok();
    let github_auth_configured = state.pr_sync.auth_configured(&state);
    let pr_synced_at = state
        .store
        .get_setting("githubPrSyncedAt")?
        .filter(|value| !value.is_empty());

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
        clickup_configured: state.clickup.is_configured(),
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

fn repo_paths_match(left: &str, right: &Path) -> bool {
    let left_path = expand_tilde(left);
    left_path.canonicalize().ok() == right.canonicalize().ok()
}

fn upsert_repo_record(app: &AppHandle, state: &AppState, repo: RepoRecord) -> Result<()> {
    state.store.upsert_repo(&repo)?;
    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(app);
    Ok(())
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
    upsert_repo_record(
        &app,
        &state,
        RepoRecord {
            id: repo.id,
            path: repo.path,
            name: repo.name,
            default_branch: repo.default_branch,
            setup_commands: repo.setup_commands,
            run_command: repo.run_command,
            single_instance: repo.single_instance,
        },
    )
}

#[tauri::command]
#[specta::specta]
pub async fn set_repo_single_instance(
    app: AppHandle,
    input: SetRepoSingleInstanceInput,
    state: State<'_, AppState>,
) -> Result<()> {
    let repos = state.store.snapshot()?.repos;
    let Some(mut repo) = repos.into_iter().find(|row| row.id == input.repo_id) else {
        return Err(Error::Workspace(format!("unknown repo {}", input.repo_id)));
    };
    repo.single_instance = input.single_instance;
    upsert_repo_record(&app, &state, repo)
}

#[tauri::command]
#[specta::specta]
pub async fn set_repo_setup_commands(
    app: AppHandle,
    input: SetRepoSetupCommandsInput,
    state: State<'_, AppState>,
) -> Result<()> {
    let repos = state.store.snapshot()?.repos;
    let Some(mut repo) = repos.into_iter().find(|row| row.id == input.repo_id) else {
        return Err(Error::Workspace(format!("unknown repo {}", input.repo_id)));
    };
    repo.setup_commands = input.setup_commands;
    upsert_repo_record(
        &app,
        &state,
        RepoRecord {
            id: repo.id,
            path: repo.path,
            name: repo.name,
            default_branch: repo.default_branch,
            setup_commands: repo.setup_commands,
            run_command: repo.run_command,
            single_instance: repo.single_instance,
        },
    )
}

#[tauri::command]
#[specta::specta]
pub async fn set_repo_default_branch(
    app: AppHandle,
    input: SetRepoDefaultBranchInput,
    state: State<'_, AppState>,
) -> Result<()> {
    let snapshot = state.store.snapshot()?;
    let Some(mut repo) = snapshot
        .repos
        .into_iter()
        .find(|row| row.id == input.repo_id)
    else {
        return Err(Error::Workspace(format!("unknown repo {}", input.repo_id)));
    };
    let branch = input.default_branch.trim().to_string();
    if branch.is_empty() {
        return Err(Error::Workspace("Pick a default branch".into()));
    }
    if let Some(workspace) = snapshot
        .workspaces
        .iter()
        .find(|row| row.repo_id == repo.id && row.archived_at.is_none() && row.branch == branch)
    {
        return Err(Error::Workspace(format!(
            "{} has {branch} checked out. Switch it to another branch first.",
            workspace.name
        )));
    }
    repo.default_branch = Some(branch);
    upsert_repo_record(&app, &state, repo)
}

/// Fast-forward the repo checkout's default branch from origin.
/// Commits each repo's default branch is behind and ahead of `origin`, against the last
/// fetch. A repo with no `origin` copy of the branch reads as 0 and 0.
#[tauri::command]
#[specta::specta]
pub async fn get_repo_git(state: State<'_, AppState>) -> Result<Vec<RepoGitRuntime>> {
    let mut rows = Vec::new();
    for repo in state.store.snapshot()?.repos {
        let path = expand_tilde(&repo.path);
        let branch = repo.default_branch_or_main();
        let behind = state
            .git
            .rev_list_count(&path, &format!("{branch}..origin/{branch}"))
            .await?;
        let ahead = state
            .git
            .rev_list_count(&path, &format!("origin/{branch}..{branch}"))
            .await?;
        rows.push(RepoGitRuntime {
            repo_id: repo.id,
            behind,
            ahead,
        });
    }
    Ok(rows)
}

#[tauri::command]
#[specta::specta]
pub async fn pull_repo_default_branch(
    repo_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    let snapshot = state.store.snapshot()?;
    let repo = snapshot
        .repos
        .into_iter()
        .find(|row| row.id == repo_id)
        .ok_or_else(|| Error::Workspace(format!("unknown repo {repo_id}")))?;
    let branch = repo.default_branch_or_main().to_string();
    let pulled = state
        .git
        .pull_branch(&expand_tilde(&repo.path), &branch)
        .await?;

    // Workspaces count how far behind origin/<base> they are, which the pull just moved.
    let updates = state.fetch.tick().await?;
    crate::git_workspace::apply_behind_updates(&state, &updates).await;
    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::BehindCounts,
    }
    .emit(&app);

    let summary = match pulled {
        0 => " is up to date".to_string(),
        1 => ": pulled 1 commit".to_string(),
        count => format!(": pulled {count} commits"),
    };
    emit_toast(
        &app,
        crate::ipc::types::ToastRaisedPayload {
            tone: crate::ipc::types::ToastTone::Ok,
            parts: vec![
                crate::ipc::types::ToastPart::Text {
                    value: format!("{} ", repo.name),
                },
                crate::ipc::types::ToastPart::Code { value: branch },
                crate::ipc::types::ToastPart::Text { value: summary },
            ],
            workspace_id: None,
            thread_id: None,
        },
    );
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn add_repo(
    app: AppHandle,
    input: AddRepoInput,
    state: State<'_, AppState>,
) -> Result<RepoRecord> {
    let (normalized, id) = crate::harness_config::validate_add_path(&input.path)?;
    let expanded = expand_tilde(&normalized);
    if !expanded.is_dir() {
        return Err(Error::Git(format!(
            "folder does not exist: {}",
            expanded.display()
        )));
    }

    let toplevel = state
        .git
        .show_toplevel(&expanded)
        .await
        .map_err(|_| Error::Git("That folder is not a git repository".into()))?;
    let display_path = crate::harness_config::path_for_display(&toplevel);

    let snapshot = state.store.snapshot()?;
    if snapshot
        .repos
        .iter()
        .any(|row| repo_paths_match(&row.path, &toplevel))
    {
        return Err(Error::Workspace("That folder is already added".into()));
    }
    if snapshot.repos.iter().any(|row| row.id == id) {
        return Err(Error::Workspace(format!(
            "A repo named {id} is already added"
        )));
    }

    let default_branch = state.git.default_branch(&toplevel).await.ok();
    let committed = crate::harness_config::read_committed_config(&toplevel);
    let suggested = crate::harness_config::suggest_commands(&toplevel);
    let (setup_commands, run_command) =
        crate::harness_config::merge_initial_config(committed, suggested);

    let name = id.clone();
    let record = RepoRecord {
        id,
        path: display_path.clone(),
        name,
        default_branch,
        setup_commands,
        run_command,
        single_instance: false,
    };
    upsert_repo_record(&app, &state, record.clone())?;

    emit_toast(
        &app,
        crate::ipc::types::ToastRaisedPayload {
            tone: crate::ipc::types::ToastTone::Ok,
            parts: vec![crate::ipc::types::ToastPart::Text {
                value: format!(
                    "Added {}. Add its setup and run commands below.",
                    record.name
                ),
            }],
            workspace_id: None,
            thread_id: None,
        },
    );
    resync_prs(&app);

    Ok(record)
}

#[tauri::command]
#[specta::specta]
pub async fn remove_repo(
    app: AppHandle,
    input: RemoveRepoInput,
    state: State<'_, AppState>,
) -> Result<()> {
    let snapshot = state.store.snapshot()?;
    let Some(repo) = snapshot
        .repos
        .into_iter()
        .find(|row| row.id == input.repo_id)
    else {
        return Err(Error::Workspace(format!("unknown repo {}", input.repo_id)));
    };

    let used = state.store.count_active_workspaces_for_repo(&repo.id)?;
    if used > 0 {
        let tear = if used == 1 { "it" } else { "them" };
        return Err(Error::Workspace(format!(
            "{} has {used} workspace{}. Tear {tear} down first.",
            repo.name,
            if used == 1 { "" } else { "s" }
        )));
    }
    if state.store.repo_count()? <= 1 {
        return Err(Error::Workspace("Harness needs at least one repo".into()));
    }

    state.store.delete_repo(&repo.id)?;
    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(&app);

    emit_toast(
        &app,
        crate::ipc::types::ToastRaisedPayload {
            tone: crate::ipc::types::ToastTone::Ok,
            parts: vec![crate::ipc::types::ToastPart::Text {
                value: format!("Removed {}. The folder on disk wasn't touched.", repo.name),
            }],
            workspace_id: None,
            thread_id: None,
        },
    );
    resync_prs(&app);
    Ok(())
}

/// The PR list only shows registered repos, so re-sync when that set changes.
fn resync_prs(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = tauri::Manager::state::<AppState>(&app);
        if let Err(error) = state.pr_sync.sync_app(&app).await {
            log::warn!("PR sync after repo change failed: {error}");
        }
    });
}

#[tauri::command]
#[specta::specta]
pub async fn test_repo_setup(
    input: TestRepoSetupInput,
    state: State<'_, AppState>,
) -> Result<TestRepoSetupResult> {
    let snapshot = state.store.snapshot()?;
    let repo = snapshot
        .repos
        .into_iter()
        .find(|row| row.id == input.repo_id)
        .ok_or_else(|| Error::Workspace(format!("unknown repo {}", input.repo_id)))?;
    let repo_path = expand_tilde(&repo.path);
    let setup_raw = crate::harness_config::effective_setup(&repo.setup_commands, &repo_path);
    let commands = crate::provisioning::parse_setup_commands(&setup_raw);
    if commands.is_empty() {
        return Ok(TestRepoSetupResult {
            ok: true,
            message: "No setup commands to run.".into(),
        });
    }

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let harness_env = vec![
        ("HARNESS_REPO_PATH".into(), repo.path.clone()),
        (
            "HARNESS_BRANCH".into(),
            repo.default_branch.clone().unwrap_or_else(|| "main".into()),
        ),
    ];

    for (index, command) in commands.iter().enumerate() {
        let session_id = format!("test-setup-{}-{index}", repo.id);
        state.process.spawn_session(
            &session_id,
            &shell,
            &["-l", "-c", command],
            Some(&repo_path),
            &harness_env,
        )?;
        let deadline = std::time::Instant::now() + crate::provisioning::SETUP_COMMAND_TIMEOUT;
        loop {
            if let Some(code) = state.process.exit_code(&session_id) {
                if code != 0 {
                    return Ok(TestRepoSetupResult {
                        ok: false,
                        message: format!("Command failed ({code}): {command}"),
                    });
                }
                break;
            }
            if std::time::Instant::now() >= deadline {
                let _ = state.process.stop_session(&session_id);
                return Ok(TestRepoSetupResult {
                    ok: false,
                    message: format!("Command timed out: {command}"),
                });
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    Ok(TestRepoSetupResult {
        ok: true,
        message: format!(
            "Ran {} setup command{} in the repo checkout.",
            commands.len(),
            if commands.len() == 1 { "" } else { "s" }
        ),
    })
}

#[tauri::command]
#[specta::specta]
pub async fn set_setting(
    app: AppHandle,
    input: SetSettingInput,
    state: State<'_, AppState>,
) -> Result<()> {
    let key = input.key.trim();
    if key.is_empty() {
        return Err(Error::Store("setting key required".into()));
    }
    state.store.set_setting(key, &input.value)?;
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

fn emit_settings_changed(app: &AppHandle, state: &AppState) {
    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(app);
}

/// Checks the key with ClickUp, then saves it to `~/.cormux/credentials.json`.
#[tauri::command]
#[specta::specta]
pub async fn set_clickup_api_key(
    app: AppHandle,
    key: String,
    state: State<'_, AppState>,
) -> Result<()> {
    state.clickup.save_key(&key).await?;
    emit_settings_changed(&app, &state);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn clear_clickup_api_key(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    state.clickup.clear_key()?;
    emit_settings_changed(&app, &state);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn clickup_workspaces(state: State<'_, AppState>) -> Result<Vec<ClickupOption>> {
    state.clickup.workspaces().await
}

#[tauri::command]
#[specta::specta]
pub async fn clickup_spaces(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ClickupOption>> {
    state.clickup.spaces(&workspace_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn clickup_folders(
    space_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ClickupOption>> {
    state.clickup.folders(&space_id).await
}

/// The current sprint in the sprint folder chosen in Settings.
#[tauri::command]
#[specta::specta]
pub async fn clickup_board(state: State<'_, AppState>) -> Result<ClickupBoard> {
    let folder_id = state
        .store
        .get_setting(CLICKUP_FOLDER_KEY)?
        .filter(|value| !value.is_empty())
        .ok_or_else(|| Error::Clickup("choose a sprint folder in Settings".into()))?;
    state.clickup.board(&folder_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn clickup_task(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<ClickupTaskDetail> {
    state.clickup.task(&task_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn set_clickup_task_status(
    task_id: String,
    status: String,
    state: State<'_, AppState>,
) -> Result<()> {
    state.clickup.set_status(&task_id, &status).await
}

#[tauri::command]
#[specta::specta]
pub async fn set_clickup_task_points(
    task_id: String,
    points: f64,
    state: State<'_, AppState>,
) -> Result<()> {
    state.clickup.set_points(&task_id, points).await
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
) -> Result<u32> {
    let engines = state.engines.clone();
    let mut sub = state.subscriptions.register();
    let id = sub.id;
    tauri::async_runtime::spawn(async move {
        let Some(mut rx) = subscribe_thread(&engines, &thread_id, &mut sub).await else {
            return;
        };
        loop {
            let event = tokio::select! {
                _ = sub.stopped() => break,
                event = rx.recv() => event,
            };
            let Ok(event) = event else { break };
            if let crate::engines::AgentEvent::MessageChunk { text, .. } = event {
                let _ = channel.send(AgentChunk {
                    thread_id: thread_id.clone(),
                    text,
                });
            }
        }
    });
    Ok(id)
}

/// High-volume ordered stream of normalised agent events for one thread.
#[tauri::command]
#[specta::specta]
pub fn subscribe_agent_events(
    thread_id: String,
    channel: Channel<AgentEvent>,
    state: State<'_, AppState>,
) -> Result<u32> {
    let engines = state.engines.clone();
    let mut sub = state.subscriptions.register();
    let id = sub.id;
    tauri::async_runtime::spawn(async move {
        // A new session replaces the engine, so when one closes wait for the next.
        while let Some(mut rx) = subscribe_thread(&engines, &thread_id, &mut sub).await {
            loop {
                let event = tokio::select! {
                    _ = sub.stopped() => return,
                    event = rx.recv() => event,
                };
                match event {
                    Ok(event) => {
                        let _ = channel.send(event);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    });
    Ok(id)
}

/// Wait for the thread's engine session to exist, giving up if the subscription is cancelled first.
async fn subscribe_thread(
    engines: &crate::engines::EngineRegistry,
    thread_id: &str,
    sub: &mut SubscriptionHandle,
) -> Option<tokio::sync::broadcast::Receiver<crate::engines::AgentEvent>> {
    loop {
        if let Ok(rx) = engines.subscribe(thread_id) {
            return Some(rx);
        }
        tokio::select! {
            _ = sub.stopped() => return None,
            _ = tokio::time::sleep(Duration::from_millis(50)) => {}
        }
    }
}

/// Stop the background task behind a `subscribe_*` channel.
#[tauri::command]
#[specta::specta]
pub fn unsubscribe(id: u32, state: State<'_, AppState>) -> Result<()> {
    state.subscriptions.cancel(id);
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
    if !approved && let Some(reason) = deny_message.filter(|value| !value.trim().is_empty()) {
        let _ = state
            .engines
            .prompt(&row.thread_id, format!("Approval denied: {reason}"));
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
) -> Result<u32> {
    let process = state.process.clone();
    let sub = state.subscriptions.register();
    let id = sub.id;
    tauri::async_runtime::spawn(async move {
        // Replay the log so reopening the Output tab shows what already ran.
        let mut lines = process.take_log_snapshot(&workspace_id);
        while !sub.is_stopped() {
            // Run and setup sessions are forwarded into the workspace log by their monitors.
            lines.extend(process.drain_pending(&workspace_id));
            for line in lines.drain(..) {
                let _ = channel.send(PtyChunk {
                    workspace_id: workspace_id.clone(),
                    line,
                });
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    });
    Ok(id)
}

/// High-volume ordered stream of live diff updates for a worktree.
#[tauri::command]
#[specta::specta]
pub async fn subscribe_diffs(
    workspace_id: String,
    channel: Channel<DiffUpdate>,
    state: State<'_, AppState>,
) -> Result<u32> {
    if let Err(error) = crate::git_workspace::load_workspace(&state, &workspace_id).await {
        log::warn!("diff watch for {workspace_id} failed: {error}");
    }
    let diffs = state.diffs.clone();
    let sub = state.subscriptions.register();
    let id = sub.id;
    tauri::async_runtime::spawn(async move {
        let mut last = None;
        while !sub.is_stopped() {
            if let Some(diff) = diffs.latest(&workspace_id)
                && last.as_ref() != Some(&diff)
            {
                let _ = channel.send(DiffUpdate {
                    workspace_id: workspace_id.clone(),
                    path: String::new(),
                    diff: Some(diff.clone()),
                });
                last = Some(diff);
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    });
    Ok(id)
}

/// Force-refresh the live diff snapshot for one workspace (branch switch, pull, etc.).
#[tauri::command]
#[specta::specta]
pub async fn refresh_workspace_diff(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<()> {
    use std::path::Path;

    let workspace = crate::git_workspace::load_workspace(&state, &workspace_id).await?;
    if let Ok(snapshot) = state.store.snapshot()
        && let Some(row) = snapshot
            .workspaces
            .iter()
            .find(|row| row.id == workspace_id)
        && row.kind.as_deref() == Some("review")
    {
        state.diffs.default_diff_target(
            &workspace_id,
            crate::git::DiffTarget::head_against(workspace.base.clone()),
        );
    }
    state
        .diffs
        .compute(&workspace_id, Path::new(&workspace.worktree_path))
        .await?;
    Ok(())
}

/// Point the Changes panel at uncommitted work (`None`) or a branch against its base.
#[tauri::command]
#[specta::specta]
pub async fn set_workspace_diff_target(
    workspace_id: String,
    target: Option<crate::git::DiffTarget>,
    state: State<'_, AppState>,
) -> Result<()> {
    use std::path::Path;

    let workspace = crate::git_workspace::load_workspace(&state, &workspace_id).await?;
    state.diffs.set_diff_target(&workspace_id, target);
    state
        .diffs
        .compute(&workspace_id, Path::new(&workspace.worktree_path))
        .await?;
    Ok(())
}

/// Throw away uncommitted changes to `paths` (all of them when empty), then refresh Changes.
#[tauri::command]
#[specta::specta]
pub async fn discard_workspace_changes(
    workspace_id: String,
    paths: Vec<String>,
    state: State<'_, AppState>,
) -> Result<()> {
    use std::path::Path;

    let workspace = crate::git_workspace::load_workspace(&state, &workspace_id).await?;
    let worktree = Path::new(&workspace.worktree_path);
    state.git.discard(worktree, &paths).await?;
    state.diffs.compute(&workspace_id, worktree).await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn create_review_workspace(
    input: crate::review::CreateReviewWorkspaceInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::review::CreateReviewWorkspaceResult> {
    crate::review::create_review_workspace(&app, &state, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn submit_workspace_review(
    input: crate::review::SubmitWorkspaceReviewInput,
    app: AppHandle,
) -> Result<()> {
    crate::review::submit_workspace_review(&app, input).await
}

/// Start a Reviewer on a regular workspace's own branch. Returns its thread id.
#[tauri::command]
#[specta::specta]
pub async fn start_branch_review(
    workspace_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String> {
    let thread_id = crate::review::start_branch_review(&app, &state, &workspace_id).await?;
    emit_composer_snapshot(&app, &state);
    Ok(thread_id)
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

    if let (Some(summary), Some(at)) = (&workspace.summary, &workspace.summary_at)
        && !summary.is_empty()
    {
        return Ok(WorkspaceSummaryResult {
            workspace_id,
            summary: summary.clone(),
            summary_at: at.clone(),
            summary_source: workspace.summary_source.clone(),
            from_llm: true,
        });
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
        _ => (summaries::fallback_summary(&workspace, &threads), false),
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
        summary_at: updated.summary_at.unwrap_or_else(|| "now".into()),
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
    // Branches that only exist on GitHub can be checked out too; `git switch` tracks them.
    names.extend(state.git.list_remote_branches(&repo_path, "origin").await?);
    names.sort();
    names.dedup();
    Ok(RepoBranchesResult { branches: names })
}

#[tauri::command]
#[specta::specta]
pub async fn list_repo_local_branches(
    repo_id: String,
    state: State<'_, AppState>,
) -> Result<LocalBranchesResult> {
    let snapshot = state.store.snapshot()?;
    let repo = snapshot
        .repos
        .into_iter()
        .find(|row| row.id == repo_id)
        .ok_or_else(|| Error::Git(format!("unknown repo {repo_id}")))?;
    let repo_path = expand_tilde(&repo.path);
    let head = state
        .git
        .current_branch(&repo_path)
        .await
        .unwrap_or_default();
    let branches = state
        .git
        .list_local_branch_tips(&repo_path)
        .await?
        .into_iter()
        .map(|tip| LocalBranchRow {
            name: tip.name,
            committed: tip.committed,
            subject: tip.subject,
        })
        .collect();
    Ok(LocalBranchesResult { head, branches })
}

#[tauri::command]
#[specta::specta]
pub async fn delete_local_branch(
    repo_id: String,
    branch: String,
    force: bool,
    state: State<'_, AppState>,
) -> Result<BranchDeletion> {
    let branch = branch.trim();
    if branch.is_empty() {
        return Err(Error::Git("missing branch name".into()));
    }
    let snapshot = state.store.snapshot()?;
    let repo = snapshot
        .repos
        .iter()
        .find(|row| row.id == repo_id)
        .ok_or_else(|| Error::Git(format!("unknown repo {repo_id}")))?;
    if branch == repo.default_branch_or_main() {
        return Err(Error::Git(format!("{branch} is the default branch")));
    }
    if let Some(workspace) = snapshot
        .workspaces
        .iter()
        .find(|row| row.repo_id == repo_id && row.branch == branch)
    {
        return Err(Error::Git(format!(
            "{branch} is checked out by {}",
            workspace.name
        )));
    }
    let repo_path = expand_tilde(&repo.path);
    let head = state
        .git
        .current_branch(&repo_path)
        .await
        .unwrap_or_default();
    if !head.is_empty() && head == branch {
        return Err(Error::Git(format!("{branch} is checked out in the repo")));
    }
    if force {
        state.git.branch_delete(&repo_path, branch).await?;
        return Ok(BranchDeletion::Deleted);
    }
    Ok(
        if state.git.delete_merged_branch(&repo_path, branch).await? {
            BranchDeletion::Deleted
        } else {
            BranchDeletion::NotMerged
        },
    )
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
    repo.ensure_not_default_branch(&input.branch)?;
    let repo_path = expand_tilde(&repo.path);
    if !repo_path.is_dir() {
        return Err(Error::Git(format!(
            "repo path does not exist: {}",
            repo_path.display()
        )));
    }

    // The dialog sends no name; each new workspace is numbered.
    let name = match input.name.trim() {
        "" => crate::naming::next_workspace_name(
            snapshot.workspaces.iter().map(|row| row.name.as_str()),
        ),
        given => given.chars().take(48).collect(),
    };
    let input = CreateWorkspaceInput { name, ..input };

    let workspace_id = new_workspace_id();
    let thread_id = Uuid::new_v4().to_string();
    let repo_name = repo_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo");
    let worktrees_base = worktrees_base(&state.store)?;
    let worktree_path = crate::workspace::WorkspaceManager::worktree_path(
        &worktrees_base,
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
            &worktrees_base,
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
        pr_html_url: None,
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
    let goal = input.goal.trim().to_string();
    if !goal.is_empty() {
        let message = serde_json::json!({ "role": "user", "text": &goal });
        state
            .store
            .append_event(&thread_id, "message", &message.to_string())?;
    }
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
    let workspace_id_bg = workspace_id.clone();
    let thread_id_bg = thread_id.clone();
    let engine = input.engine.clone();
    let repo_name = repo.name.clone();
    let setup_commands = crate::harness_config::effective_setup(&repo.setup_commands, &repo_path);

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
            thread_id: Some(thread_id.clone()),
        },
    );

    tauri::async_runtime::spawn(async move {
        crate::provisioning::run_workspace_provisioning(
            app_handle,
            crate::provisioning::LeadProvisionJob {
                workspace_id: workspace_id_bg,
                thread_id: thread_id_bg,
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
pub async fn retry_workspace_provisioning(workspace_id: String, app: AppHandle) -> Result<()> {
    tauri::async_runtime::spawn(async move {
        crate::provisioning::retry_provisioning(app, workspace_id).await;
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn skip_workspace_provisioning_setup(workspace_id: String, app: AppHandle) -> Result<()> {
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
    state.store.set_workspace_name(&input.workspace_id, name)?;
    state.workspace.rename(&input.workspace_id, name).await?;
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

/// The PR prompt Settings shows until the user writes their own.
#[tauri::command]
#[specta::specta]
pub fn default_pr_prompt() -> String {
    crate::pr_draft::DEFAULT_PR_PROMPT.into()
}

#[tauri::command]
#[specta::specta]
pub async fn draft_pr_why(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<DraftPrWhyResult> {
    crate::create_pr::draft_pr_why(&state, &workspace_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn create_workspace_pull_request(
    input: CreateWorkspacePullRequestInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<CreateWorkspacePullRequestResult> {
    crate::create_pr::create_workspace_pull_request(&app, &state, input).await
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
    crate::git_workspace::create_workspace_branch(&app, &state, &input.workspace_id, &input.branch)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn get_workspace_stack(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<crate::stack::WorkspaceStack> {
    crate::stack::workspace_stack(&state, &workspace_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn add_stack_branch(
    input: CreateWorkspaceBranchInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::stack::add_branch(&app, &state, &input.workspace_id, &input.branch).await
}

#[tauri::command]
#[specta::specta]
pub async fn push_stack(
    workspace_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::stack::push(&app, &state, &workspace_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn sync_stack(
    workspace_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::stack::sync(&app, &state, &workspace_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn get_teardown_preview(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<TeardownPreview> {
    crate::teardown::preview(&state, &workspace_id).await
}

/// Open the worktree in the terminal app picked in Settings (Terminal by default).
#[tauri::command]
#[specta::specta]
pub async fn open_workspace_terminal(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<()> {
    let row = state
        .store
        .workspace_by_id(&workspace_id)?
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

/// Close a thread tab: stop its agent and hide it. The workspace's first thread stays open.
#[tauri::command]
#[specta::specta]
pub async fn close_workspace_thread(
    app: AppHandle,
    thread_id: String,
    state: State<'_, AppState>,
) -> Result<()> {
    let thread = snapshot_thread(&state, &thread_id)?;
    if state.store.scratch_for_thread(&thread_id)?.is_some() {
        return Err(Error::Workspace(
            "a scratch's thread can't be closed".into(),
        ));
    }
    if state
        .store
        .first_thread_id(&thread.workspace_id)?
        .as_deref()
        == Some(thread_id.as_str())
    {
        return Err(Error::Workspace(format!(
            "{} can't be closed",
            thread.title
        )));
    }
    let _ = state.engines.stop(&thread_id).await;
    state.store.close_thread(&thread_id)?;
    state
        .workspace
        .remove_thread(&thread_id, &thread.workspace_id)
        .await;
    emit_composer_snapshot(&app, &state);
    Ok(())
}

/// Skills the `/` picker offers for this thread's agent.
#[tauri::command]
#[specta::specta]
pub async fn list_thread_skills(
    thread_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::skills::Skill>> {
    crate::skills::for_thread(&state, &thread_id)
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
    if let Some(scratch) = state.store.scratch_for_thread(&thread_id)? {
        return crate::scratch::send(&app, &state, &scratch, text).await;
    }
    let thread = snapshot_thread(&state, &thread_id)?;
    let held = thread.status == "paused";
    if !held && !state.engines.has_thread(&thread_id) {
        crate::provisioning::ensure_thread_engine(
            &state,
            &thread_id,
            &thread.engine,
            &thread.workspace_id,
        )
        .await?;
    }
    persist_user_message(&state.store, &thread_id, &text)?;
    // A failed send must not mark the thread running: nothing would ever end that turn.
    state.engines.submit_prompt(&thread_id, text, held)?;
    if !held {
        state.store.set_thread_status(&thread_id, "running")?;
        state
            .workspace
            .set_thread(&thread_id, &thread.workspace_id, ThreadActivity::Running)
            .await;
        state
            .store
            .set_workspace_status(&thread.workspace_id, "running")?;
    }
    emit_composer_snapshot(&app, &state);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_thread_turn(
    app: AppHandle,
    thread_id: String,
    state: State<'_, AppState>,
) -> Result<()> {
    let thread = snapshot_thread(&state, &thread_id).ok();
    let _ = state.engines.cancel(&thread_id);
    let _ = persist_control_step(&state.store, &thread_id, "You stopped the agent");
    let _ = state.store.mark_thread_idle(&thread_id);
    if let Some(thread) = thread {
        set_workspace_thread(&state, &thread, ThreadActivity::Idle).await;
    }
    emit_composer_snapshot(&app, &state);
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
    set_workspace_thread(&state, &thread, ThreadActivity::Paused).await;
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
    set_workspace_thread(&state, &thread, ThreadActivity::Running).await;
    if let Some(text) = state.engines.release_thread(&thread_id)? {
        let _ = state.engines.prompt(&thread_id, text);
    }
    emit_composer_snapshot(&app, &state);
    Ok(())
}

/// The models the thread's composer can pick from, and the current pick.
#[tauri::command]
#[specta::specta]
pub async fn thread_models(thread_id: String, state: State<'_, AppState>) -> Result<ThreadModels> {
    let thread = snapshot_thread(&state, &thread_id)?;
    state
        .engines
        .thread_models(&thread_id, EngineKind::from_name(&thread.engine))
}

/// `None` goes back to the engine's default. A live engine switches before its next turn.
#[tauri::command]
#[specta::specta]
pub async fn set_thread_model(
    thread_id: String,
    model: Option<String>,
    state: State<'_, AppState>,
) -> Result<()> {
    snapshot_thread(&state, &thread_id)?;
    state.engines.set_thread_model(&thread_id, model)
}

/// Like `/clear`: the agent forgets the conversation, but the thread keeps showing it.
#[tauri::command]
#[specta::specta]
pub async fn new_thread_session(
    app: AppHandle,
    thread_id: String,
    state: State<'_, AppState>,
) -> Result<()> {
    let thread = snapshot_thread(&state, &thread_id)?;
    if state.store.scratch_for_thread(&thread_id)?.is_some() {
        return Err(Error::Workspace(
            "a scratch can't start a new session".into(),
        ));
    }
    state.engines.discard(&thread_id)?;
    state.store.clear_thread_session(&thread_id)?;
    persist_control_step(&state.store, &thread_id, "Started a new session")?;
    state.store.mark_thread_idle(&thread_id)?;
    set_workspace_thread(&state, &thread, ThreadActivity::Idle).await;
    emit_composer_snapshot(&app, &state);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn send_workspace_findings(
    app: AppHandle,
    input: SendWorkspaceFindingsInput,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::findings::send_workspace_findings(
        &app,
        &input.workspace_id,
        &input.thread_id,
        input.finding_ids,
    )
    .await?;
    emit_composer_snapshot(&app, &state);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn create_scratch(
    app: AppHandle,
    input: super::types::CreateScratchInput,
    state: State<'_, AppState>,
) -> Result<super::types::CreateScratchResult> {
    crate::scratch::create(&app, &state, input).await
}

/// Discards the scratch's conversation. The repo is not changed.
#[tauri::command]
#[specta::specta]
pub async fn end_scratch(
    app: AppHandle,
    scratch_id: String,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::scratch::end(&app, &state, &scratch_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn create_todo(title: String, state: State<'_, AppState>) -> Result<TodoRow> {
    let title = title.trim();
    if title.is_empty() {
        return Err(Error::Store("a todo needs a title".into()));
    }
    let todo = TodoRow {
        id: format!("todo-{}", Uuid::new_v4()),
        title: title.to_string(),
        pinned: false,
    };
    state.store.insert_todo(&todo)?;
    Ok(todo)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_todo(todo_id: String, state: State<'_, AppState>) -> Result<()> {
    state.store.delete_todo(&todo_id)
}

/// Pinned todos show as cards at the top of Homebase.
#[tauri::command]
#[specta::specta]
pub async fn set_todo_pinned(
    todo_id: String,
    pinned: bool,
    state: State<'_, AppState>,
) -> Result<()> {
    state.store.set_todo_pinned(&todo_id, pinned)
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

pub fn harness_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn worktrees_base(store: &crate::store::Store) -> Result<PathBuf> {
    Ok(store
        .get_setting("worktreeRoot")?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(|value| expand_tilde(&value))
        .unwrap_or_else(|| {
            crate::workspace::WorkspaceManager::default_worktrees_base(&harness_home())
        }))
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(rest);
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

/// Any thread, a scratch's included.
fn snapshot_thread(state: &AppState, thread_id: &str) -> Result<ThreadRow> {
    state
        .store
        .thread_by_id(thread_id)?
        .ok_or_else(|| Error::Store(format!("unknown thread {thread_id}")))
}

/// Workspace lifecycle bookkeeping. A scratch thread has no workspace, so it is skipped.
async fn set_workspace_thread(state: &AppState, thread: &ThreadRow, activity: ThreadActivity) {
    if state
        .store
        .scratch_for_thread(&thread.id)
        .ok()
        .flatten()
        .is_some()
    {
        return;
    }
    state
        .workspace
        .set_thread(&thread.id, &thread.workspace_id, activity)
        .await;
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
