use std::collections::BTreeMap;

use tauri::{State, ipc::Channel};

use super::subscriptions::{Subscriptions, stream_agent_events, stream_diffs, stream_pty};
use super::types::{
    ControlWorkspaceAppInput, CreateWorkspaceBranchInput, CreateWorkspaceResult, DiffUpdate,
    PtyChunk, SetSettingInput, Snapshot, SwitchWorkspaceBranchInput, TeardownInput,
    TeardownPreview, WorkspaceAppControlAction,
};
use crate::app::WorkspaceAppAction;
use crate::clickup::types::{ClickupBoard, ClickupOption, ClickupTaskDetail};
use crate::create_pr::{
    CreateWorkspacePullRequestInput, CreateWorkspacePullRequestResult, DraftPrWhyResult,
};
use crate::engines::{EngineKind, StreamedEvent, ThreadModels};
use crate::error::{Error, Result};
use crate::feedback::Change;
use crate::findings::{DismissWorkspaceFindingsInput, SendWorkspaceFindingsInput};
use crate::model::WorkspaceKind;
use crate::provisioning::ProvisionPlan;
use crate::repos::{
    AddRepoInput, BranchDeletion, LocalBranchesResult, RemoveRepoInput, RepoBranchesResult,
    RepoGitRuntime, SetRepoDefaultBranchInput, SetRepoRunCommandInput, SetRepoSetupCommandsInput,
    SetRepoSingleInstanceInput, TestRepoSetupInput, TestRepoSetupResult,
};
use crate::state::AppState;
use crate::store::types::RepoRecord;
use crate::summaries::WorkspaceSummaryResult;
use crate::turn::{ResumeThreadSessionInput, thread_row};
use crate::workspace_start::CreateWorkspaceInput;
use crate::workspaces::RenameWorkspaceInput;

#[tauri::command]
#[specta::specta]
pub async fn get_snapshot(state: State<'_, AppState>) -> Result<Snapshot> {
    let memory = state.metrics.latest();
    let github_auth_configured = state.pr_sync.auth_configured(&state);

    let persisted = state.store.blocking(|store| store.snapshot()).await?;
    let workspaces = state
        .workspaces
        .records_for(&persisted.workspaces, &persisted.repos)
        .await;
    let workspace_ids: Vec<String> = workspaces.iter().map(|row| row.id.clone()).collect();
    let workspace_git = state.workspace_git.snapshot(&workspace_ids).await;
    let repo_run_commands = crate::harness_config::repo_run_commands(&persisted.repos);

    Ok(Snapshot {
        version: state.out.version(),
        persisted,
        workspaces,
        workspace_git,
        memory,
        pending_live_approvals: state.approvals.pending_count(),
        github_auth_configured,
        clickup_configured: state.clickup.is_configured(),
        workspace_apps: state.apps.snapshot(),
        repo_run_commands,
    })
}

/// Whether git would take `name` as a branch name; the error says why not.
#[tauri::command]
#[specta::specta]
pub fn validate_branch_name(name: String) -> Result<()> {
    crate::git::validate_ref_name(name.trim(), "branch")
}

#[tauri::command]
#[specta::specta]
pub async fn control_workspace_app(
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
        .control(&state, &input.workspace_id, action)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn set_repo_run_command(
    input: SetRepoRunCommandInput,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::repos::set_run_command(&state, input)
}

#[tauri::command]
#[specta::specta]
pub async fn set_repo_single_instance(
    input: SetRepoSingleInstanceInput,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::repos::set_single_instance(&state, input)
}

#[tauri::command]
#[specta::specta]
pub async fn set_repo_setup_commands(
    input: SetRepoSetupCommandsInput,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::repos::set_setup_commands(&state, input)
}

#[tauri::command]
#[specta::specta]
pub async fn set_repo_default_branch(
    input: SetRepoDefaultBranchInput,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::repos::set_default_branch(&state, input)
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
pub async fn pull_repo_default_branch(repo_id: String, state: State<'_, AppState>) -> Result<()> {
    crate::repos::pull_default_branch(&state, &repo_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn add_repo(input: AddRepoInput, state: State<'_, AppState>) -> Result<RepoRecord> {
    crate::repos::add(&state, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn remove_repo(input: RemoveRepoInput, state: State<'_, AppState>) -> Result<()> {
    crate::repos::remove(&state, input).await
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
pub async fn set_setting(input: SetSettingInput, state: State<'_, AppState>) -> Result<()> {
    let key = input.key.trim();
    if key.is_empty() {
        return Err(Error::Store("setting key required".into()));
    }
    state.store.set_setting(key, &input.value)?;
    state.out.changed(Change::THREADS);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn reload_environment(state: State<'_, AppState>) -> Result<()> {
    state.shell_env.write().await.load().await?;
    state.out.changed(Change::SNAPSHOT);
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

/// Checks the key with ClickUp, then saves it to `~/.cormux/credentials.json`.
#[tauri::command]
#[specta::specta]
pub async fn set_clickup_api_key(key: String, state: State<'_, AppState>) -> Result<()> {
    state.clickup.save_key(&key).await?;
    state.out.changed(Change::THREADS);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn clear_clickup_api_key(state: State<'_, AppState>) -> Result<()> {
    state.clickup.clear_key()?;
    state.out.changed(Change::THREADS);
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
pub async fn sync_pull_requests(state: State<'_, AppState>) -> Result<()> {
    state.pr_sync.sync_app(&state).await
}

/// High-volume ordered stream of normalised agent events for one thread.
#[tauri::command]
#[specta::specta]
pub fn subscribe_agent_events(
    thread_id: String,
    channel: Channel<StreamedEvent>,
    state: State<'_, AppState>,
    subs: State<'_, Subscriptions>,
) -> Result<u32> {
    Ok(stream_agent_events(&state, &subs, thread_id, channel))
}
/// Stop the background task behind a `subscribe_*` channel.
#[tauri::command]
#[specta::specta]
pub fn unsubscribe(id: u32, subs: State<'_, Subscriptions>) -> Result<()> {
    subs.cancel(id);
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
    id: String,
    approved: bool,
    deny_reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::approvals::resolve_approval(&state, &id, approved, deny_reason).await
}
#[tauri::command]
#[specta::specta]
pub async fn answer_question(
    id: String,
    answers: BTreeMap<String, String>,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::approvals::answer_question(&state, &id, answers)
}

#[tauri::command]
#[specta::specta]
pub async fn resolve_all_approvals(
    thread_id: String,
    approved: bool,
    deny_reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::approvals::resolve_all_approvals(&state, &thread_id, approved, deny_reason).await
}

/// High-volume ordered stream of PTY output for a workspace's run/setup log.
#[tauri::command]
#[specta::specta]
pub fn subscribe_pty(
    workspace_id: String,
    channel: Channel<PtyChunk>,
    state: State<'_, AppState>,
    subs: State<'_, Subscriptions>,
) -> Result<u32> {
    Ok(stream_pty(&state, &subs, workspace_id, channel))
}

/// High-volume ordered stream of live diff updates for a worktree.
#[tauri::command]
#[specta::specta]
pub async fn subscribe_diffs(
    workspace_id: String,
    channel: Channel<DiffUpdate>,
    state: State<'_, AppState>,
    subs: State<'_, Subscriptions>,
) -> Result<u32> {
    Ok(stream_diffs(&state, &subs, workspace_id, channel).await)
}

/// Force-refresh the live diff snapshot for one workspace (branch switch, pull, etc.).
#[tauri::command]
#[specta::specta]
pub async fn refresh_workspace_diff(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::changes::refresh(&state, &workspace_id).await
}

/// Point the Changes panel at uncommitted work (`None`) or a branch against its base.
#[tauri::command]
#[specta::specta]
pub async fn set_workspace_diff_target(
    workspace_id: String,
    target: Option<crate::git::DiffTarget>,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::changes::set_target(&state, &workspace_id, target).await
}

/// Throw away uncommitted changes to `paths` (all of them when empty), then refresh Changes.
#[tauri::command]
#[specta::specta]
pub async fn discard_workspace_changes(
    workspace_id: String,
    paths: Vec<String>,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::changes::discard(&state, &workspace_id, &paths).await
}

#[tauri::command]
#[specta::specta]
pub async fn create_review_workspace(
    input: crate::review::CreateReviewWorkspaceInput,
    state: State<'_, AppState>,
) -> Result<crate::review::CreateReviewWorkspaceResult> {
    crate::review::create_review_workspace(&state, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn submit_workspace_review(
    input: crate::review::SubmitWorkspaceReviewInput,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::review::submit_workspace_review(&state, input).await
}

/// Start a Reviewer on a regular workspace's own branch. Returns its thread id.
#[tauri::command]
#[specta::specta]
pub async fn start_branch_review(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<String> {
    let thread_id = crate::review::start_branch_review(&state, &workspace_id).await?;
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
    state: State<'_, AppState>,
) -> Result<CreateWorkspaceResult> {
    let started = crate::workspace_start::start_workspace(&state, input, None).await?;
    Ok(CreateWorkspaceResult {
        workspace_id: started.workspace_id,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn retry_workspace_provisioning(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<()> {
    let plan = resume_plan(&state, &workspace_id).await?;
    let state = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = crate::provisioning::retry(&state, plan).await {
            log::warn!("retrying setup of {workspace_id}: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn skip_workspace_provisioning_setup(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<()> {
    let plan = resume_plan(&state, &workspace_id).await?;
    let state = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = crate::provisioning::skip_setup(&state, plan).await {
            log::warn!("skipping setup of {workspace_id}: {error}");
        }
    });
    Ok(())
}

/// How a workspace whose setup failed is brought up again, by whoever knows its kind.
async fn resume_plan(state: &AppState, workspace_id: &str) -> Result<ProvisionPlan> {
    let workspace = state.workspaces.get(workspace_id).await?;
    match workspace.kind {
        WorkspaceKind::Lead => crate::workspace_start::resume_plan(state, &workspace),
        WorkspaceKind::PrReview => crate::review::resume_plan(state, &workspace),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn rename_workspace(
    state: State<'_, AppState>,
    input: RenameWorkspaceInput,
) -> Result<()> {
    state
        .workspaces
        .rename(&input.workspace_id, &input.name)
        .await?;
    state.out.changed(Change::THREADS);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn switch_workspace_branch(
    input: SwitchWorkspaceBranchInput,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::workspace_git::switch_branch(&state, &input.workspace_id, &input.branch).await
}

#[tauri::command]
#[specta::specta]
pub async fn pull_workspace(workspace_id: String, state: State<'_, AppState>) -> Result<()> {
    crate::workspace_git::pull(&state, &workspace_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn rebase_workspace(workspace_id: String, state: State<'_, AppState>) -> Result<()> {
    crate::workspace_git::rebase(&state, &workspace_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn push_workspace_branch(workspace_id: String, state: State<'_, AppState>) -> Result<()> {
    crate::workspace_git::push(&state, &workspace_id).await
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
    state: State<'_, AppState>,
) -> Result<CreateWorkspacePullRequestResult> {
    crate::create_pr::create_workspace_pull_request(&state, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn abort_workspace_git(workspace_id: String, state: State<'_, AppState>) -> Result<()> {
    crate::workspace_git::abort_conflict(&state, &workspace_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn create_workspace_branch(
    input: CreateWorkspaceBranchInput,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::workspace_git::create_branch(&state, &input.workspace_id, &input.branch).await
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
    state: State<'_, AppState>,
) -> Result<()> {
    crate::stack::add_branch(&state, &input.workspace_id, &input.branch).await
}

#[tauri::command]
#[specta::specta]
pub async fn push_stack(workspace_id: String, state: State<'_, AppState>) -> Result<()> {
    crate::stack::push(&state, &workspace_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn sync_stack(workspace_id: String, state: State<'_, AppState>) -> Result<()> {
    crate::stack::sync(&state, &workspace_id).await
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
    crate::terminal::open(&state, &workspace_id).await
}
#[tauri::command]
#[specta::specta]
pub async fn teardown_workspace(input: TeardownInput, state: State<'_, AppState>) -> Result<()> {
    crate::teardown::execute(&state, &input).await
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
    thread_id: String,
    text: String,
    attachments: Vec<crate::attachments::PromptAttachment>,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::turn::submit(&state, &thread_id, text, attachments).await
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_thread_turn(thread_id: String, state: State<'_, AppState>) -> Result<()> {
    crate::turn::stop(&state, &thread_id).await;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn pause_thread(thread_id: String, state: State<'_, AppState>) -> Result<()> {
    crate::turn::pause(&state, &thread_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn resume_thread(thread_id: String, state: State<'_, AppState>) -> Result<()> {
    crate::turn::resume(&state, &thread_id).await
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
pub async fn new_thread_session(thread_id: String, state: State<'_, AppState>) -> Result<()> {
    crate::turn::new_session(&state, &thread_id).await
}

/// Picks an earlier conversation in the thread back up: the agent resumes its
/// session, and the thread shows that conversation again. `conversation_id`
/// is the UI's id for it, echoed back in the step so the timeline can follow.
#[tauri::command]
#[specta::specta]
pub async fn resume_thread_session(
    input: ResumeThreadSessionInput,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::turn::resume_session(&state, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn send_workspace_findings(
    input: SendWorkspaceFindingsInput,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::findings::send_workspace_findings(
        &state,
        &input.workspace_id,
        &input.thread_id,
        input.finding_ids,
    )
    .await?;
    Ok(())
}

/// Copies a media file an agent referenced in its reply into the app's media folder,
/// where the webview may load it. `None` when it isn't a showable image or recording.
#[tauri::command]
#[specta::specta]
pub async fn resolve_thread_media(
    source: String,
    workdir: Option<String>,
    state: State<'_, AppState>,
) -> Result<Option<crate::thread_media::ThreadMedia>> {
    let media_dir = state.data_dir.media();
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
    input: DismissWorkspaceFindingsInput,
    state: State<'_, AppState>,
) -> Result<()> {
    crate::findings::dismiss_findings(&state, input)
}

/// Discards the scratch's conversation. The repo is not changed.
#[tauri::command]
#[specta::specta]
pub async fn end_scratch(scratch_id: String, state: State<'_, AppState>) -> Result<()> {
    crate::scratch::end(&state, &scratch_id).await
}
