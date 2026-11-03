use std::collections::BTreeMap;

use tauri::{AppHandle, State, ipc::Channel};

use crate::app::WorkspaceAppAction;
use crate::clickup::types::{ClickupBoard, ClickupOption, ClickupTaskDetail};
use crate::engines::{EngineKind, StreamedEvent, ThreadModels};
use crate::error::{Error, Result};
use crate::feedback::Output;
use crate::ipc::events::output;
use crate::ipc::subscriptions::{stream_agent_events, stream_diffs, stream_pty};
use crate::ipc::types::StateChangeKind;
use crate::state::AppState;
use crate::store::types::RepoRecord;
use crate::turn::thread_row;

use super::types::{
    AddRepoInput, BranchDeletion, ControlWorkspaceAppInput, CreateWorkspaceBranchInput,
    CreateWorkspaceInput, CreateWorkspacePullRequestInput, CreateWorkspacePullRequestResult,
    CreateWorkspaceResult, DiffUpdate, DraftPrWhyResult, LocalBranchesResult, PtyChunk,
    RemoveRepoInput, RenameWorkspaceInput, RepoBranchesResult, RepoGitRuntime,
    ResolveApprovalResult, ResumeThreadSessionInput, SendWorkspaceFindingsInput,
    SetRepoDefaultBranchInput, SetRepoRunCommandInput, SetRepoSetupCommandsInput,
    SetRepoSingleInstanceInput, SetSettingInput, Snapshot, SwitchWorkspaceBranchInput,
    TeardownInput, TeardownPreview, TestRepoSetupInput, TestRepoSetupResult,
    WorkspaceAppControlAction, WorkspaceSummaryResult,
};

#[tauri::command]
#[specta::specta]
pub async fn get_snapshot(state: State<'_, AppState>) -> Result<Snapshot> {
    let memory = state.metrics.latest();
    let github_auth_configured = state.pr_sync.auth_configured(&state);
    let pr_synced_at = state
        .store
        .get_setting("githubPrSyncedAt")?
        .filter(|value| !value.is_empty());

    let persisted = state.store.blocking(|store| store.snapshot()).await?;
    let workspaces = state.workspace.list().await;
    let workspace_ids: Vec<String> = if workspaces.is_empty() {
        persisted
            .workspaces
            .iter()
            .filter(|row| row.archived_at.is_none())
            .map(|row| row.id.clone())
            .collect()
    } else {
        workspaces.iter().map(|row| row.id.clone()).collect()
    };
    let workspace_git = crate::git_workspace::git_runtime_snapshot(&state, &workspace_ids).await;

    Ok(Snapshot {
        version: state.snapshot_version(),
        view: super::types::AppView::Homebase,
        persisted,
        workspaces,
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
        .control(&output(&app), &state, &input.workspace_id, action)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn set_repo_run_command(
    app: AppHandle,
    input: SetRepoRunCommandInput,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::repos::set_run_command(&output(&app), &state, input)
}

#[tauri::command]
#[specta::specta]
pub async fn set_repo_single_instance(
    app: AppHandle,
    input: SetRepoSingleInstanceInput,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::repos::set_single_instance(&output(&app), &state, input)
}

#[tauri::command]
#[specta::specta]
pub async fn set_repo_setup_commands(
    app: AppHandle,
    input: SetRepoSetupCommandsInput,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::repos::set_setup_commands(&output(&app), &state, input)
}

#[tauri::command]
#[specta::specta]
pub async fn set_repo_default_branch(
    app: AppHandle,
    input: SetRepoDefaultBranchInput,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::repos::set_default_branch(&output(&app), &state, input)
}

/// Commits each repo's default branch is behind and ahead of `origin`, against the last
/// fetch. A repo with no `origin` copy of the branch reads as 0 and 0.
#[tauri::command]
#[specta::specta]
pub async fn get_repo_git(state: State<'_, AppState>) -> Result<Vec<RepoGitRuntime>> {
    crate::repos::git_runtime(&state).await
}

/// Fast-forward the repo checkout's default branch from origin.
#[tauri::command]
#[specta::specta]
pub async fn pull_repo_default_branch(
    repo_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::repos::pull_default_branch(&output(&app), &state, &repo_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn add_repo(
    app: AppHandle,
    input: AddRepoInput,
    state: State<'_, AppState>,
) -> Result<RepoRecord> {
    crate::repos::add(&output(&app), &state, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn remove_repo(
    app: AppHandle,
    input: RemoveRepoInput,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::repos::remove(&output(&app), &state, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn test_repo_setup(
    input: TestRepoSetupInput,
    state: State<'_, AppState>,
) -> Result<TestRepoSetupResult> {
    crate::repos::test_setup(&state, input).await
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
    state.notify(&output(&app), StateChangeKind::WorkspaceStatus);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn reload_environment(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    state.shell_env.write().await.reload().await?;
    state.notify(&output(&app), StateChangeKind::Environment);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn get_metrics(state: State<'_, AppState>) -> Result<crate::metrics::MemorySample> {
    match state.metrics.latest() {
        Some(sample) => Ok(sample),
        None => crate::metrics::sample_processes(&state).await,
    }
}
/// A window of a thread's transcript: its newest rows by default, those after `after_seq` to
/// catch up, or those before `before_seq` to scroll back.
#[tauri::command]
#[specta::specta]
pub async fn get_thread_events(
    thread_id: String,
    after_seq: Option<i64>,
    before_seq: Option<i64>,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<crate::store::types::ThreadEventPage> {
    state
        .store
        .blocking(move |store| store.thread_event_page(&thread_id, after_seq, before_seq, limit))
        .await
}

fn emit_settings_changed(out: &Output, state: &AppState) {
    state.notify(out, StateChangeKind::WorkspaceStatus);
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
    emit_settings_changed(&output(&app), &state);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn clear_clickup_api_key(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    state.clickup.clear_key()?;
    emit_settings_changed(&output(&app), &state);
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
    state.clickup.chosen_board(&state.store).await
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
    state.pr_sync.sync_app(&output(&app), &state).await
}

/// High-volume ordered stream of normalised agent events for one thread.
#[tauri::command]
#[specta::specta]
pub fn subscribe_agent_events(
    thread_id: String,
    channel: Channel<StreamedEvent>,
    state: State<'_, AppState>,
) -> Result<u32> {
    Ok(stream_agent_events(&state, thread_id, channel))
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
    crate::approvals::resolve_approval(&output(&app), &state, &id, approved, deny_reason).await
}
#[tauri::command]
#[specta::specta]
pub async fn answer_question(
    app: AppHandle,
    id: String,
    answers: BTreeMap<String, String>,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::approvals::answer_question(&output(&app), &state, &id, answers)
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
    crate::approvals::resolve_all_approvals(
        &output(&app),
        &state,
        &thread_id,
        approved,
        deny_reason,
    )
    .await
}

/// High-volume ordered stream of PTY output for a workspace's run/setup log.
#[tauri::command]
#[specta::specta]
pub fn subscribe_pty(
    workspace_id: String,
    channel: Channel<PtyChunk>,
    state: State<'_, AppState>,
) -> Result<u32> {
    Ok(stream_pty(&state, workspace_id, channel))
}

/// High-volume ordered stream of live diff updates for a worktree.
#[tauri::command]
#[specta::specta]
pub async fn subscribe_diffs(
    workspace_id: String,
    channel: Channel<DiffUpdate>,
    state: State<'_, AppState>,
) -> Result<u32> {
    Ok(stream_diffs(&state, workspace_id, channel).await)
}

/// Force-refresh the live diff snapshot for one workspace (branch switch, pull, etc.).
#[tauri::command]
#[specta::specta]
pub async fn refresh_workspace_diff(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::git_workspace::refresh_diff(&state, &workspace_id).await
}

/// Point the Changes panel at uncommitted work (`None`) or a branch against its base.
#[tauri::command]
#[specta::specta]
pub async fn set_workspace_diff_target(
    workspace_id: String,
    target: Option<crate::git::DiffTarget>,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::git_workspace::set_diff_target(&state, &workspace_id, target).await
}

/// Throw away uncommitted changes to `paths` (all of them when empty), then refresh Changes.
#[tauri::command]
#[specta::specta]
pub async fn discard_workspace_changes(
    workspace_id: String,
    paths: Vec<String>,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::git_workspace::discard_changes(&state, &workspace_id, &paths).await
}

#[tauri::command]
#[specta::specta]
pub async fn create_review_workspace(
    input: crate::review::CreateReviewWorkspaceInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::review::CreateReviewWorkspaceResult> {
    crate::review::create_review_workspace(&output(&app), &state, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn submit_workspace_review(
    input: crate::review::SubmitWorkspaceReviewInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::review::submit_workspace_review(&output(&app), &state, input).await
}

/// Start a Reviewer on a regular workspace's own branch. Returns its thread id.
#[tauri::command]
#[specta::specta]
pub async fn start_branch_review(
    workspace_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String> {
    let out = output(&app);
    let thread_id = crate::review::start_branch_review(&out, &state, &workspace_id).await?;
    emit_composer_snapshot(&out, &state);
    Ok(thread_id)
}

/// Generate or return a cached workspace card summary (debounced LLM + local fallback).
#[tauri::command]
#[specta::specta]
pub async fn summarise_workspace(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<WorkspaceSummaryResult> {
    crate::summaries::summarise(&state, workspace_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn list_repo_branches(
    repo_id: String,
    state: State<'_, AppState>,
) -> Result<RepoBranchesResult> {
    crate::repos::list_branches(&state, &repo_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn list_repo_local_branches(
    repo_id: String,
    state: State<'_, AppState>,
) -> Result<LocalBranchesResult> {
    crate::repos::list_local_branches(&state, &repo_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_local_branch(
    repo_id: String,
    branch: String,
    force: bool,
    state: State<'_, AppState>,
) -> Result<BranchDeletion> {
    crate::repos::delete_local_branch(&state, &repo_id, &branch, force).await
}

#[tauri::command]
#[specta::specta]
pub async fn create_workspace(
    input: CreateWorkspaceInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<CreateWorkspaceResult> {
    let started =
        crate::workspace_start::start_workspace(&output(&app), &state, input, None).await?;
    Ok(CreateWorkspaceResult {
        workspace_id: started.workspace_id,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn retry_workspace_provisioning(
    workspace_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    let (out, state) = (output(&app), state.inner().clone());
    tauri::async_runtime::spawn(async move {
        crate::provisioning::retry_provisioning(out, state, workspace_id).await;
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn skip_workspace_provisioning_setup(
    workspace_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    let (out, state) = (output(&app), state.inner().clone());
    tauri::async_runtime::spawn(async move {
        crate::provisioning::skip_provisioning_setup(out, state, workspace_id).await;
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
    crate::git_workspace::rename_workspace(&output(&app), &state, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn switch_workspace_branch(
    input: SwitchWorkspaceBranchInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::git_workspace::switch_workspace_branch(
        &output(&app),
        &state,
        &input.workspace_id,
        &input.branch,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn pull_workspace(
    workspace_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::git_workspace::pull_workspace(&output(&app), &state, &workspace_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn rebase_workspace(
    workspace_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::git_workspace::rebase_workspace(&output(&app), &state, &workspace_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn push_workspace_branch(
    workspace_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::git_workspace::push_workspace_branch(&output(&app), &state, &workspace_id).await
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
    crate::create_pr::create_workspace_pull_request(&output(&app), &state, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn abort_workspace_git(
    workspace_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::git_workspace::abort_workspace_git_conflict(&output(&app), &state, &workspace_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn create_workspace_branch(
    input: CreateWorkspaceBranchInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::git_workspace::create_workspace_branch(
        &output(&app),
        &state,
        &input.workspace_id,
        &input.branch,
    )
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
    crate::stack::add_branch(&output(&app), &state, &input.workspace_id, &input.branch).await
}

#[tauri::command]
#[specta::specta]
pub async fn push_stack(
    workspace_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::stack::push(&output(&app), &state, &workspace_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn sync_stack(
    workspace_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::stack::sync(&output(&app), &state, &workspace_id).await
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
    crate::git_workspace::open_terminal(&state, &workspace_id).await
}
#[tauri::command]
#[specta::specta]
pub async fn teardown_workspace(
    input: TeardownInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::teardown::execute(&output(&app), &state, &input).await
}

/// Skills the `/` picker offers on the new thread page.
#[tauri::command]
#[specta::specta]
pub async fn list_repo_skills(
    repo_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::skills::Skill>> {
    crate::skills::for_repo(&state, &repo_id).await
}

/// Skills the `/` picker offers for this thread's agent.
#[tauri::command]
#[specta::specta]
pub async fn list_thread_skills(
    thread_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::skills::Skill>> {
    crate::skills::for_thread(&state, &thread_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn send_thread_prompt(
    app: AppHandle,
    thread_id: String,
    text: String,
    attachments: Vec<crate::attachments::PromptAttachment>,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::turn::submit(&output(&app), &state, &thread_id, text, attachments).await
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_thread_turn(
    app: AppHandle,
    thread_id: String,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::turn::stop(&output(&app), &state, &thread_id).await;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn pause_thread(
    app: AppHandle,
    thread_id: String,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::turn::pause(&output(&app), &state, &thread_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn resume_thread(
    app: AppHandle,
    thread_id: String,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::turn::resume(&output(&app), &state, &thread_id).await
}

/// The models the thread's composer can pick from, and the current pick.
#[tauri::command]
#[specta::specta]
pub async fn thread_models(thread_id: String, state: State<'_, AppState>) -> Result<ThreadModels> {
    let thread = thread_row(&state, &thread_id)?;
    state
        .engines
        .thread_models(&thread_id, EngineKind::from_name(&thread.engine))
}

/// `None` goes back to the engine's default. A live engine switches before its next turn.
/// The models an engine offers, for a thread that doesn't exist yet.
#[tauri::command]
#[specta::specta]
pub async fn engine_models(engine: String) -> Result<Vec<crate::engines::ModelOption>> {
    Ok(crate::engines::static_models(EngineKind::from_name(
        &engine,
    )))
}

#[tauri::command]
#[specta::specta]
pub async fn set_thread_model(
    thread_id: String,
    model: Option<String>,
    state: State<'_, AppState>,
) -> Result<()> {
    thread_row(&state, &thread_id)?;
    state.engines.set_thread_model(&thread_id, model)
}

/// The effort levels the thread's composer can pick from, and the current pick.
#[tauri::command]
#[specta::specta]
pub async fn thread_efforts(thread_id: String, state: State<'_, AppState>) -> Result<ThreadModels> {
    let thread = thread_row(&state, &thread_id)?;
    state
        .engines
        .thread_efforts(&thread_id, EngineKind::from_name(&thread.engine))
}

/// The effort levels an engine offers, for a thread that doesn't exist yet.
#[tauri::command]
#[specta::specta]
pub async fn engine_efforts(engine: String) -> Result<Vec<crate::engines::ModelOption>> {
    Ok(crate::engines::static_efforts(EngineKind::from_name(
        &engine,
    )))
}

/// `None` goes back to the engine's default. A live engine switches before its next turn.
#[tauri::command]
#[specta::specta]
pub async fn set_thread_effort(
    thread_id: String,
    effort: Option<String>,
    state: State<'_, AppState>,
) -> Result<()> {
    thread_row(&state, &thread_id)?;
    state.engines.set_thread_effort(&thread_id, effort)
}

/// Like `/clear`: the agent forgets the conversation and the thread starts a
/// fresh one. The step records the session it ended, so the conversation can
/// be resumed later (`resume_thread_session`).
#[tauri::command]
#[specta::specta]
pub async fn new_thread_session(
    app: AppHandle,
    thread_id: String,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::turn::new_session(&output(&app), &state, &thread_id).await
}

/// Picks an earlier conversation in the thread back up: the agent resumes its
/// session, and the thread shows that conversation again. `conversation_id`
/// is the UI's id for it, echoed back in the step so the timeline can follow.
#[tauri::command]
#[specta::specta]
pub async fn resume_thread_session(
    app: AppHandle,
    input: ResumeThreadSessionInput,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::turn::resume_session(&output(&app), &state, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn send_workspace_findings(
    app: AppHandle,
    input: SendWorkspaceFindingsInput,
    state: State<'_, AppState>,
) -> Result<()> {
    let out = output(&app);
    crate::findings::send_workspace_findings(
        &out,
        &state,
        &input.workspace_id,
        &input.thread_id,
        input.finding_ids,
    )
    .await?;
    emit_composer_snapshot(&out, &state);
    Ok(())
}

/// Copies a media file an agent referenced in its reply into the app's media folder,
/// where the webview may load it. `None` when it isn't a showable image or recording.
#[tauri::command]
#[specta::specta]
pub async fn resolve_thread_media(
    app: AppHandle,
    source: String,
    workdir: Option<String>,
) -> Result<Option<crate::thread_media::ThreadMedia>> {
    use tauri::Manager;
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| Error::Workspace(error.to_string()))?;
    let media_dir = crate::data_dir(app_data).join("media");
    // Copies a file of up to hundreds of megabytes; not on an async worker.
    tokio::task::spawn_blocking(move || {
        crate::thread_media::resolve(
            &media_dir,
            &source,
            workdir.as_deref().map(std::path::Path::new),
        )
    })
    .await
    .map_err(|error| Error::Workspace(error.to_string()))?
}

/// A thumbnail for an image attached by path, for the composer.
#[tauri::command]
#[specta::specta]
pub async fn image_preview(path: String) -> Result<Option<String>> {
    // Up to 5 MB read and base64-encoded: not on an async worker.
    tokio::task::spawn_blocking(move || {
        crate::attachments::image_preview(std::path::Path::new(&path))
    })
    .await
    .map_err(|error| Error::Workspace(error.to_string()))
}

/// Dismissed findings stay listed but aren't sent to an agent or posted with the review.
#[tauri::command]
#[specta::specta]
pub async fn dismiss_workspace_findings(
    app: AppHandle,
    input: super::types::DismissWorkspaceFindingsInput,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::findings::dismiss_findings(&output(&app), &state, input)
}

/// Discards the scratch's conversation. The repo is not changed.
#[tauri::command]
#[specta::specta]
pub async fn end_scratch(
    app: AppHandle,
    scratch_id: String,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::scratch::end(&output(&app), &state, &scratch_id).await
}

fn emit_composer_snapshot(out: &Output, state: &AppState) {
    state.notify(out, StateChangeKind::WorkspaceStatus);
}
