use std::path::Path;

use serde::Deserialize;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::feedback::emit_toast;
use crate::ipc::types::{ToastPart, ToastRaisedPayload, ToastTone};
use crate::git::WorktreeDiff;
use crate::github::auth;
use crate::github::review::{
    RestGithubClient, ReviewLineComment, ReviewVerdict, SubmitPullRequestReviewInput,
};
use crate::ipc::commands::{expand_tilde, worktrees_base};
use crate::ipc::events::StateChanged;
use crate::ipc::types::StateChangeKind;
use crate::llm::LlmClient;
use crate::provisioning::LeadProvisionJob;
use crate::state::AppState;
use crate::store::Store;
use crate::store::types::{FindingRow, ThreadRow, WorkspaceRow};
use crate::workspace::{ThreadActivity, WorkspaceLifecycle, WorkspaceRecord};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateReviewWorkspaceInput {
    pub repo_id: String,
    pub pr_number: i64,
    pub title: String,
    pub head: String,
    pub base: String,
    pub author: String,
    pub author_is_you: bool,
    pub files_changed: i64,
    pub pr_html_url: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateReviewWorkspaceResult {
    pub workspace_id: String,
    pub created: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum SubmitReviewVerdict {
    Approve,
    RequestChanges,
    Comment,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SubmitWorkspaceReviewInput {
    pub workspace_id: String,
    pub verdict: SubmitReviewVerdict,
}

pub fn review_status_key(workspace_id: &str) -> String {
    format!("review:{workspace_id}/status")
}

pub fn review_author_key(workspace_id: &str) -> String {
    format!("review:{workspace_id}/author")
}

pub fn review_submitted_key(workspace_id: &str) -> String {
    format!("review:{workspace_id}/submitted")
}

pub fn user_review_message(
    pr_number: i64,
    title: &str,
    author: &str,
    author_is_you: bool,
) -> String {
    let by = if author_is_you {
        "you".to_string()
    } else {
        format!("@{author}")
    };
    format!(
        "Review #{pr_number} “{title}” by {by}. Flag bugs, risky changes and missing tests. Don’t push commits; draft comments for me to approve."
    )
}

pub fn reviewer_rubric() -> &'static str {
    "You are the Reviewer for this pull request. Read the PR description and diff carefully. \
     Look for bugs, risky changes, and missing tests. Do not push commits or post to GitHub. \
     For each issue, call the Harness MCP tool `report_finding` with severity (blocking, suggestion, or nit), \
     title, file path, line number, and explanation. When finished, call `finish_review`."
}

pub fn reviewer_engine_prompt(user_message: &str) -> String {
    format!("{user_message}\n\n{}", reviewer_rubric())
}

pub fn find_existing_review_workspace(
    store: &Store,
    repo_id: &str,
    pr_number: i64,
) -> Option<WorkspaceRow> {
    let snapshot = store.snapshot().ok()?;
    snapshot.workspaces.into_iter().find(|row| {
        row.archived_at.is_none()
            && row.kind.as_deref() == Some("review")
            && row.repo_id == repo_id
            && row.pr_number == Some(pr_number)
    })
}

fn default_engine(store: &Store) -> String {
    store
        .get_setting("defaultEngine")
        .ok()
        .flatten()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "claude".into())
}

fn workspace_id_for_pr(pr_number: i64) -> String {
    format!("pr{pr_number}")
}

pub async fn create_review_workspace(
    app: &AppHandle,
    state: &AppState,
    input: CreateReviewWorkspaceInput,
) -> Result<CreateReviewWorkspaceResult> {
    if let Some(existing) =
        find_existing_review_workspace(&state.store, &input.repo_id, input.pr_number)
    {
        remember_workspace_from_row(state, &existing, &input.base).await?;
        return Ok(CreateReviewWorkspaceResult {
            workspace_id: existing.id,
            created: false,
        });
    }

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

    let workspace_id = workspace_id_for_pr(input.pr_number);
    if snapshot.workspaces.iter().any(|row| row.id == workspace_id) {
        return Err(Error::Workspace(format!(
            "workspace id {workspace_id} is already in use"
        )));
    }

    let thread_id = Uuid::new_v4().to_string();
    let engine = default_engine(&state.store);
    let repo_name = repo_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo")
        .to_string();
    let worktrees_root = worktrees_base(&state.store)?;
    let worktree_path = crate::workspace::WorkspaceManager::worktree_path(
        &worktrees_root,
        &repo_name,
        &input.head,
    );

    state
        .workspace
        .register_provisioning(
            &workspace_id,
            &input.repo_id,
            &repo_path,
            &input.title,
            &input.head,
            &input.base,
            &worktrees_root,
        )
        .await?;

    state.store.upsert_workspace(&WorkspaceRow {
        id: workspace_id.clone(),
        repo_id: input.repo_id.clone(),
        name: input.title.clone(),
        branch: input.head.clone(),
        worktree_path: worktree_path.to_string_lossy().to_string(),
        status: "provisioning".into(),
        created_at: String::new(),
        summary: None,
        summary_at: None,
        summary_source: "Haiku 4.5".into(),
        kind: Some("review".into()),
        pr_number: Some(input.pr_number),
        pr_html_url: input.pr_html_url.clone(),
        modified_files: 0,
        archived_at: None,
    })?;

    let user_message = user_review_message(
        input.pr_number,
        &input.title,
        &input.author,
        input.author_is_you,
    );
    state.store.append_event(
        &thread_id,
        "message",
        &serde_json::json!({ "role": "user", "text": user_message }).to_string(),
    )?;

    let checkout_detail = format!(
        "{} → {} · {} files changed",
        input.head, input.base, input.files_changed
    );
    let checkout_step = serde_json::json!({
        "icon": "pr",
        "title": format!("Checked out #{}", input.pr_number),
        "detail": checkout_detail,
    });
    state
        .store
        .append_event(&thread_id, "tool", &checkout_step.to_string())?;

    state.store.upsert_thread(&ThreadRow {
        id: thread_id.clone(),
        workspace_id: workspace_id.clone(),
        title: "Reviewer".into(),
        engine: engine.clone(),
        session_id: None,
        status: "provisioning".into(),
        used_tokens: None,
        context_size: None,
        cost_usd: None,
        transcript_readonly: false,
    })?;

    let author_label = if input.author_is_you {
        "you".into()
    } else {
        format!("@{}", input.author)
    };
    let _ = state
        .store
        .set_setting(&review_author_key(&workspace_id), &author_label);

    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(app);

    emit_toast(
        app,
        ToastRaisedPayload {
            tone: ToastTone::Ok,
            parts: vec![
                ToastPart::Text {
                    value: "Opened a review workspace for ".into(),
                },
                ToastPart::Code {
                    value: format!("#{}", input.pr_number),
                },
            ],
            workspace_id: Some(workspace_id.clone()),
        },
    );

    let app_handle = app.clone();
    let goal = user_message.clone();
    let pr_number = input.pr_number as u64;
    let base_branch = input.base.clone();
    let repo_id_bg = input.repo_id.clone();
    let setup_commands = repo.setup_commands.clone();
    let worktree_path_bg = worktree_path.clone();
    let workspace_id_bg = workspace_id.clone();

    tauri::async_runtime::spawn(async move {
        let state = app_handle.state::<AppState>();
        if let Err(error) = state
            .workspace
            .add_review_worktree(&workspace_id_bg, pr_number)
            .await
        {
            log::warn!("review worktree failed: {error}");
            return;
        }
        state
            .diffs
            .set_pr_diff_base(&workspace_id_bg, base_branch.clone());
        let _ = state
            .diffs
            .compute(&workspace_id_bg, &worktree_path_bg)
            .await;

        crate::provisioning::run_workspace_provisioning(
            app_handle.clone(),
            LeadProvisionJob {
                workspace_id: workspace_id_bg.clone(),
                thread_id: thread_id.clone(),
                repo_id: repo_id_bg,
                repo_name,
                setup_commands_raw: setup_commands,
                engine,
                goal,
                review: true,
            },
        )
        .await;

        schedule_reviewer_pass(app_handle, workspace_id_bg, thread_id).await;
    });

    Ok(CreateReviewWorkspaceResult {
        workspace_id,
        created: true,
    })
}

async fn remember_workspace_from_row(
    state: &AppState,
    row: &WorkspaceRow,
    base: &str,
) -> Result<()> {
    let snapshot = state.store.snapshot()?;
    let repo = snapshot
        .repos
        .iter()
        .find(|entry| entry.id == row.repo_id)
        .ok_or_else(|| Error::Git(format!("unknown repo {}", row.repo_id)))?;
    let repo_path = expand_tilde(&repo.path);
    state
        .workspace
        .remember(WorkspaceRecord {
            id: row.id.clone(),
            repo_id: row.repo_id.clone(),
            repo_path: repo_path.to_string_lossy().to_string(),
            name: row.name.clone(),
            branch: row.branch.clone(),
            base: base.to_string(),
            worktree_path: row.worktree_path.clone(),
            status: parse_lifecycle(&row.status),
            version: 1,
            activity: String::new(),
            prov_step: 0,
            setup_failed_command: None,
            setup_failed_exit_code: None,
        })
        .await;
    state.diffs.set_pr_diff_base(&row.id, base.to_string());
    Ok(())
}

fn parse_lifecycle(raw: &str) -> WorkspaceLifecycle {
    match raw {
        "provisioning" => WorkspaceLifecycle::Provisioning,
        "running" => WorkspaceLifecycle::Running,
        "idle" => WorkspaceLifecycle::Idle,
        "waiting" => WorkspaceLifecycle::Waiting,
        "provisioningFailed" => WorkspaceLifecycle::ProvisioningFailed,
        _ => WorkspaceLifecycle::Ready,
    }
}

pub async fn schedule_reviewer_pass(app: AppHandle, workspace_id: String, thread_id: String) {
    tokio::time::sleep(std::time::Duration::from_secs(6)).await;
    let state = app.state::<AppState>();
    loop {
        if review_is_ready(&state.store, &workspace_id) {
            return;
        }
        if !reviewer_is_paused(&state, &thread_id).await {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    if let Err(error) = run_structured_review(&app, &workspace_id, &thread_id).await {
        log::warn!("structured review failed: {error}");
    }
}

async fn reviewer_is_paused(state: &AppState, thread_id: &str) -> bool {
    let snapshot = match state.store.snapshot() {
        Ok(snapshot) => snapshot,
        Err(_) => return false,
    };
    snapshot
        .threads
        .iter()
        .find(|row| row.id == thread_id)
        .is_some_and(|row| row.status == "paused")
}

pub fn review_is_ready(store: &Store, workspace_id: &str) -> bool {
    store
        .get_setting(&review_status_key(workspace_id))
        .ok()
        .flatten()
        .as_deref()
        == Some("ready")
}

pub async fn run_structured_review(
    app: &AppHandle,
    workspace_id: &str,
    thread_id: &str,
) -> Result<()> {
    let state = app.state::<AppState>();
    if review_is_ready(&state.store, workspace_id) {
        return Ok(());
    }
    let snapshot = state.store.snapshot()?;
    let workspace = snapshot
        .workspaces
        .iter()
        .find(|row| row.id == workspace_id)
        .cloned()
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
    let record = state
        .workspace
        .get(workspace_id)
        .await
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;

    let diff = state
        .diffs
        .compute(workspace_id, Path::new(&record.worktree_path))
        .await?;

    let existing = snapshot
        .findings
        .iter()
        .filter(|row| row.workspace_id == workspace_id)
        .count();
    if existing == 0 {
        seed_findings_from_diff(&state.llm, &state.store, &workspace, &diff).await?;
    }

    complete_review(app, workspace_id, thread_id).await
}

async fn seed_findings_from_diff(
    llm: &LlmClient,
    store: &Store,
    workspace: &WorkspaceRow,
    diff: &WorktreeDiff,
) -> Result<()> {
    if diff.files.is_empty() {
        return Ok(());
    }
    let prompt = build_findings_prompt(workspace, diff);
    let parsed = match llm.complete(&prompt).await {
        Ok(result) => parse_findings_json(&result.text),
        Err(error) => {
            log::warn!("review LLM failed: {error}");
            Vec::new()
        }
    };
    if parsed.is_empty() {
        fallback_findings_from_diff(store, workspace, diff)?;
        return Ok(());
    }
    for item in parsed {
        store.upsert_finding(&FindingRow {
            id: Uuid::new_v4().to_string(),
            workspace_id: workspace.id.clone(),
            severity: item.severity,
            title: item.title,
            file: item.file,
            line: item.line,
            explanation: item.explanation,
            status: "open".into(),
            commit_sha: None,
            sent_to_thread_id: None,
        })?;
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct ParsedFinding {
    severity: String,
    title: String,
    file: Option<String>,
    line: Option<i64>,
    explanation: String,
}

fn parse_findings_json(text: &str) -> Vec<ParsedFinding> {
    let trimmed = text.trim();
    let Some(json_start) = trimmed.find('[') else {
        return Vec::new();
    };
    let Some(json_end) = trimmed.rfind(']') else {
        return Vec::new();
    };
    let slice = &trimmed[json_start..=json_end];
    serde_json::from_str(slice).unwrap_or_default()
}

fn build_findings_prompt(workspace: &WorkspaceRow, diff: &WorktreeDiff) -> String {
    let files: Vec<String> = diff
        .files
        .iter()
        .take(20)
        .map(|file| format!("- {} (+{} −{})", file.path, file.added, file.deleted))
        .collect();
    format!(
        "Return ONLY a JSON array (no markdown) of code review findings for this pull request workspace \"{}\". \
         Each item: {{\"severity\":\"blocking|suggestion|nit\",\"title\":\"…\",\"file\":\"path|null\",\"line\":number|null,\"explanation\":\"…\"}}. \
         Flag real risks in the diff; prefer at most 6 items.\n\nChanged files:\n{}",
        workspace.name,
        files.join("\n")
    )
}

fn fallback_findings_from_diff(
    store: &Store,
    workspace: &WorkspaceRow,
    diff: &WorktreeDiff,
) -> Result<()> {
    if diff.files.is_empty() {
        return Ok(());
    }
    let first = &diff.files[0];
    store.upsert_finding(&FindingRow {
        id: Uuid::new_v4().to_string(),
        workspace_id: workspace.id.clone(),
        severity: "suggestion".into(),
        title: format!("Review changes in {}", first.path),
        file: Some(first.path.clone()),
        line: Some(1),
        explanation: "Walk through this file for edge cases and missing tests.".into(),
        status: "open".into(),
        commit_sha: None,
        sent_to_thread_id: None,
    })?;
    Ok(())
}

pub async fn complete_review(
    app: &AppHandle,
    workspace_id: &str,
    thread_id: &str,
) -> Result<()> {
    let state = app.state::<AppState>();
    if review_is_ready(&state.store, workspace_id) {
        return Ok(());
    }
    let snapshot = state.store.snapshot()?;
    let workspace = snapshot
        .workspaces
        .iter()
        .find(|row| row.id == workspace_id)
        .cloned()
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
    let record = state
        .workspace
        .get(workspace_id)
        .await
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;

    let diff = state
        .diffs
        .latest(workspace_id)
        .unwrap_or(WorktreeDiff {
            workspace_id: workspace_id.to_string(),
            files: vec![],
        });

    let file_count = diff.files.len();
    let findings: Vec<FindingRow> = snapshot
        .findings
        .into_iter()
        .filter(|row| row.workspace_id == workspace_id)
        .collect();

    let read_step = serde_json::json!({
        "icon": "file",
        "title": format!("Read {} changed file{}", file_count, if file_count == 1 { "" } else { "s" }),
        "detail": format!("{} against {}", record.branch, record.base),
        "chip": "Read-only, auto-approved",
    });
    state
        .store
        .append_event(thread_id, "tool", &read_step.to_string())?;

    let summary_thought = findings_summary_thought(&findings);
    state.store.append_event(
        thread_id,
        "message",
        &serde_json::json!({ "role": "assistant", "text": summary_thought }).to_string(),
    )?;

    state
        .store
        .set_setting(&review_status_key(workspace_id), "ready")?;

    let author = state
        .store
        .get_setting(&review_author_key(workspace_id))?
        .unwrap_or_else(|| "the author".into());
    let pr_label = workspace
        .pr_number
        .map(|num| format!("#{num}"))
        .unwrap_or_else(|| "the PR".into());
    let counts = count_findings(&findings);
    let card_summary = if findings.is_empty() {
        format!("Reviewed {pr_label} from {author}: no findings to send.")
    } else {
        format!(
            "Reviewed {pr_label} from {author}: {counts}. Choose which to send back for fixes in the Findings tab; nothing has been posted to GitHub yet."
        )
    };

    let now = chrono_timestamp();
    state.store.upsert_workspace(&WorkspaceRow {
        summary: Some(card_summary),
        summary_at: Some(now),
        ..workspace
    })?;

    state
        .workspace
        .set_provisioning_detail(workspace_id, "Review ready", 0, None, None)
        .await?;

    let engine = snapshot
        .threads
        .iter()
        .find(|row| row.id == thread_id)
        .map(|row| row.engine.clone())
        .unwrap_or_else(|| "claude".into());

    state.store.upsert_thread(&ThreadRow {
        id: thread_id.to_string(),
        workspace_id: workspace_id.to_string(),
        title: "Reviewer".into(),
        engine,
        session_id: None,
        status: "idle".into(),
        used_tokens: None,
        context_size: None,
        cost_usd: None,
        transcript_readonly: false,
    })?;
    state
        .workspace
        .set_thread(thread_id, workspace_id, ThreadActivity::Idle)
        .await;

    let finding_count = findings.len() as i64;
    let pr_number = workspace.pr_number.unwrap_or(0);
    emit_toast(
        app,
        ToastRaisedPayload {
            tone: ToastTone::Ok,
            parts: vec![
                ToastPart::Text {
                    value: format!("Review of #{pr_number} finished · "),
                },
                ToastPart::Code {
                    value: finding_count.to_string(),
                },
                ToastPart::Text {
                    value: if finding_count == 1 {
                        " finding".into()
                    } else {
                        " findings".into()
                    },
                },
            ],
            workspace_id: Some(workspace_id.to_string()),
        },
    );

    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(app);
    Ok(())
}

fn findings_summary_thought(findings: &[FindingRow]) -> String {
    if findings.is_empty() {
        return "Done. I didn’t find anything worth flagging in this diff.".into();
    }
    let blocking = findings
        .iter()
        .filter(|row| row.severity == "blocking")
        .count();
    let suggestions = findings
        .iter()
        .filter(|row| row.severity == "suggestion")
        .count();
    let nits = findings.iter().filter(|row| row.severity == "nit").count();
    format!(
        "Done. I found {blocking} blocking issue{}, {suggestions} suggestion{} and {nits} nit{}. Pick the ones you want fixed in the Findings tab and I’ll work through them; nothing has been posted to GitHub.",
        if blocking == 1 { "" } else { "s" },
        if suggestions == 1 { "" } else { "s" },
        if nits == 1 { "" } else { "s" },
    )
}

fn count_findings(findings: &[FindingRow]) -> String {
    let blocking = findings
        .iter()
        .filter(|row| row.severity == "blocking")
        .count();
    let suggestions = findings
        .iter()
        .filter(|row| row.severity == "suggestion")
        .count();
    let nits = findings.iter().filter(|row| row.severity == "nit").count();
    if findings.is_empty() {
        return "no findings".into();
    }
    format!(
        "{blocking} blocking issue{}, {suggestions} suggestion{} and {nits} nit{}",
        if blocking == 1 { "" } else { "s" },
        if suggestions == 1 { "" } else { "s" },
        if nits == 1 { "" } else { "s" },
    )
}

pub async fn submit_workspace_review(
    app: &AppHandle,
    input: SubmitWorkspaceReviewInput,
) -> Result<()> {
    let state = app.state::<AppState>();
    if state
        .store
        .get_setting(&review_submitted_key(&input.workspace_id))?
        .as_deref()
        == Some("true")
    {
        return Err(Error::Workspace("Review already submitted".into()));
    }

    let snapshot = state.store.snapshot()?;
    let workspace = snapshot
        .workspaces
        .iter()
        .find(|row| row.id == input.workspace_id)
        .cloned()
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {}", input.workspace_id)))?;
    if workspace.kind.as_deref() != Some("review") {
        return Err(Error::Workspace("Not a review workspace".into()));
    }
    let pr_number = workspace
        .pr_number
        .ok_or_else(|| Error::Workspace("Review workspace has no PR number".into()))?;

    let repo = snapshot
        .repos
        .iter()
        .find(|row| row.id == workspace.repo_id)
        .ok_or_else(|| Error::Git(format!("unknown repo {}", workspace.repo_id)))?;
    let (owner, repo_name) = parse_repo_slug(&repo.path, workspace.pr_html_url.as_deref())?;

    let token = auth::resolve_token(&state.shell_env)
        .await
        .ok_or_else(|| Error::Github("GitHub token not configured".into()))?;

    let open_findings: Vec<FindingRow> = snapshot
        .findings
        .into_iter()
        .filter(|row| row.workspace_id == input.workspace_id && row.status == "open")
        .collect();

    let mut body_lines = Vec::new();
    let mut line_comments = Vec::new();
    for finding in &open_findings {
        if let (Some(path), Some(line)) = (finding.file.as_ref(), finding.line) {
            if !path.is_empty() && line > 0 {
                line_comments.push(ReviewLineComment {
                    path: path.clone(),
                    line,
                    body: format!("**{}** — {}", finding.title, finding.explanation),
                });
                continue;
            }
        }
        body_lines.push(format!(
            "- **{}** ({}) — {}",
            finding.title, finding.severity, finding.explanation
        ));
    }
    let body = if body_lines.is_empty() {
        "Review submitted from Cormux.".into()
    } else {
        format!(
            "Review submitted from Cormux.\n\n{}",
            body_lines.join("\n")
        )
    };

    let verdict = match input.verdict {
        SubmitReviewVerdict::Approve => ReviewVerdict::Approve,
        SubmitReviewVerdict::RequestChanges => ReviewVerdict::RequestChanges,
        SubmitReviewVerdict::Comment => ReviewVerdict::Comment,
    };

    RestGithubClient::new()
        .submit_pull_request_review(
            &token,
            &owner,
            &repo_name,
            pr_number,
            &SubmitPullRequestReviewInput {
                verdict,
                body,
                comments: line_comments,
            },
        )
        .await?;

    let reviewer_thread = snapshot
        .threads
        .iter()
        .find(|row| row.workspace_id == input.workspace_id && row.title == "Reviewer")
        .cloned()
        .ok_or_else(|| Error::Store("Reviewer thread missing".into()))?;

    state
        .store
        .set_setting(&review_submitted_key(&input.workspace_id), "true")?;

    for thread in snapshot
        .threads
        .iter()
        .filter(|row| row.workspace_id == input.workspace_id)
    {
        state.store.upsert_thread(&ThreadRow {
            status: "idle".into(),
            ..thread.clone()
        })?;
        state
            .workspace
            .set_thread(&thread.id, &input.workspace_id, ThreadActivity::Idle)
            .await;
    }

    let submitted_step = serde_json::json!({
        "icon": "pr",
        "tone": "ok",
        "title": format!("Submitted review on #{}", pr_number),
        "detail": "Posted to GitHub",
    });
    state
        .store
        .append_event(&reviewer_thread.id, "tool", &submitted_step.to_string())?;

    state
        .workspace
        .set_provisioning_detail(&input.workspace_id, "Review submitted", 0, None, None)
        .await?;

    emit_toast(
        app,
        ToastRaisedPayload {
            tone: ToastTone::Ok,
            parts: vec![
                ToastPart::Text {
                    value: "Submitted your review on ".into(),
                },
                ToastPart::Code {
                    value: format!("#{}", pr_number),
                },
            ],
            workspace_id: Some(input.workspace_id.clone()),
        },
    );

    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(app);
    Ok(())
}

fn parse_repo_slug(repo_path: &str, pr_html_url: Option<&str>) -> Result<(String, String)> {
    if let Some(url) = pr_html_url {
        if let Some(rest) = url.strip_prefix("https://github.com/") {
            let parts: Vec<&str> = rest.split('/').collect();
            if parts.len() >= 2 {
                return Ok((parts[0].to_string(), parts[1].to_string()));
            }
        }
    }
    let origin = std::process::Command::new("git")
        .args(["-C", repo_path, "remote", "get-url", "origin"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_default();
    let slug = crate::github::parse_origin_url(origin.trim())
        .ok_or_else(|| Error::Github("could not resolve repo owner/name".into()))?;
    let mut parts = slug.split('/');
    let owner = parts
        .next()
        .ok_or_else(|| Error::Github("could not resolve repo owner".into()))?
        .to_string();
    let repo_name = parts
        .next()
        .ok_or_else(|| Error::Github("could not resolve repo name".into()))?
        .to_string();
    Ok((owner, repo_name))
}

fn chrono_timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_default()
}
