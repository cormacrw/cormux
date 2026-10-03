use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::feedback::emit_toast;
use crate::github::auth;
use crate::github::review::{
    RestGithubClient, ReviewLineComment, ReviewVerdict, SubmitPullRequestReviewInput,
};
use crate::ipc::commands::{expand_tilde, worktrees_base};
use crate::ipc::events::StateChanged;
use crate::ipc::types::StateChangeKind;
use crate::ipc::types::{ToastPart, ToastRaisedPayload, ToastTone};
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

/// Sent to the engine after the user's message but never shown in the thread. The
/// findings block it asks for is parsed at the end of the turn and hidden in the UI.
pub fn reviewer_rubric() -> String {
    rubric_with_intro(
        "You are the Reviewer for this pull request. Its branch is checked out in your working directory; \
         use `gh pr diff` or git against the base branch to see what changed, and read the surrounding code. \
         Look for bugs, risky changes and missing tests. Do not edit files, push commits or post to GitHub.",
        "line number in the PR's version of the file, or null.",
    )
}

/// The rubric for reviewing a regular workspace's own branch, uncommitted work included.
pub fn branch_reviewer_rubric(base: &str) -> String {
    rubric_with_intro(
        &format!(
            "You are the Reviewer for the work in this worktree. Review everything that differs from `{base}`: \
             the commits on this branch (`git diff {base}...HEAD`) and any uncommitted changes (`git diff HEAD`, \
             plus untracked files from `git status`). Read the surrounding code. Look for bugs, risky changes \
             and missing tests. Do not edit files, commit, push or post to GitHub."
        ),
        "line number in the working tree's version of the file, or null.",
    )
}

fn rubric_with_intro(intro: &str, line_hint: &str) -> String {
    format!(
        "{intro}

Write your review for the user in plain prose. Then end your final message with every finding as a JSON \
array inside {open} tags. The user never sees this block; Cormux turns it into the Findings list. Format:

{open}
[
  {{\"severity\": \"blocking\", \"title\": \"One-line summary\", \"file\": \"src/path/to/file.ts\", \"line\": 48, \"explanation\": \"What is wrong, why it matters, and how to fix it.\"}}
]
{close}

- severity: \"blocking\" (must be fixed before merging), \"suggestion\" (worth doing in this PR) or \"nit\" (optional polish).
- file: path relative to the repository root, or null if the finding isn't about one file.
- line: {line_hint}
- Output raw JSON with no code fence. Use [] if you found nothing. Emit the block exactly once, at the very end.",
        open = crate::findings_block::OPEN_TAG,
        close = crate::findings_block::CLOSE_TAG,
    )
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

pub const REVIEWER_TITLE: &str = "Reviewer";

fn default_engine(store: &Store) -> String {
    store
        .get_setting("defaultEngine")
        .ok()
        .flatten()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "claude".into())
}

/// Unique per review: PR numbers repeat across repos, and archived rows keep their ids.
fn workspace_id_for_pr(pr_number: i64) -> String {
    let suffix = Uuid::new_v4().simple().to_string();
    format!("pr{pr_number}-{}", &suffix[..8])
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
    repo.ensure_not_default_branch(&input.head)?;
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
    let worktree_path =
        crate::workspace::WorkspaceManager::worktree_path(&worktrees_root, &repo_name, &input.head);

    let workspace_name = format!("PR #{} Review", input.pr_number);
    state
        .workspace
        .register_provisioning(
            &workspace_id,
            &input.repo_id,
            &repo_path,
            &workspace_name,
            &input.head,
            &input.base,
            &worktrees_root,
        )
        .await?;

    state.store.upsert_workspace(&WorkspaceRow {
        id: workspace_id.clone(),
        repo_id: input.repo_id.clone(),
        name: workspace_name,
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

    // Events reference the thread by foreign key, so the thread row has to exist first.
    state.store.upsert_thread(&ThreadRow {
        id: thread_id.clone(),
        workspace_id: workspace_id.clone(),
        title: REVIEWER_TITLE.into(),
        engine: engine.clone(),
        session_id: None,
        status: "provisioning".into(),
        used_tokens: None,
        context_size: None,
        cost_usd: None,
        transcript_readonly: false,
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
            thread_id: Some(thread_id.clone()),
        },
    );

    let app_handle = app.clone();
    let goal = user_message.clone();
    let pr_number = input.pr_number as u64;
    let base_branch = input.base.clone();
    let setup_commands = crate::harness_config::effective_setup(&repo.setup_commands, &repo_path);
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
        state.diffs.set_diff_target(
            &workspace_id_bg,
            Some(crate::git::DiffTarget::head_against(base_branch.clone())),
        );
        let _ = state
            .diffs
            .compute(&workspace_id_bg, &worktree_path_bg)
            .await;

        crate::provisioning::run_workspace_provisioning(
            app_handle.clone(),
            LeadProvisionJob {
                workspace_id: workspace_id_bg.clone(),
                thread_id: thread_id.clone(),
                repo_name,
                setup_commands_raw: setup_commands,
                engine,
                goal,
                review: true,
            },
        )
        .await;
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
    state
        .diffs
        .default_diff_target(&row.id, crate::git::DiffTarget::head_against(base));
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

pub fn review_is_ready(store: &Store, workspace_id: &str) -> bool {
    store
        .get_setting(&review_status_key(workspace_id))
        .ok()
        .flatten()
        .as_deref()
        == Some("ready")
}

/// Runs when any thread's turn ends. For a Reviewer whose review isn't done yet, reads
/// the findings block from its last reply and turns it into the Findings list. If the
/// block is missing or malformed, leaves a note in the thread and tries again next turn.
pub async fn on_reviewer_turn_end(app: &AppHandle, thread_id: &str) -> Result<()> {
    let state = app.state::<AppState>();
    let Some(thread) = state.store.thread_by_id(thread_id)? else {
        return Ok(());
    };
    let Some(workspace) = state.store.workspace_by_id(&thread.workspace_id)? else {
        return Ok(());
    };
    let branch_review = workspace.kind.as_deref() != Some("review");
    let pending = if branch_review {
        branch_review_running(&state.store, &workspace.id)
    } else {
        !review_is_ready(&state.store, &workspace.id)
    };
    if workspace.archived_at.is_some() || thread.title != REVIEWER_TITLE || !pending {
        return Ok(());
    }

    let reply = state.store.last_agent_reply(thread_id)?;
    let findings = match crate::findings_block::parse_findings_block(
        &reply,
        &workspace.worktree_path,
    ) {
        Ok(findings) => findings,
        Err(error) => {
            let detail = match error {
                crate::findings_block::BlockError::Missing => {
                    "The reply didn’t end with a findings block. Ask the Reviewer to finish the review with its findings.".to_string()
                }
                crate::findings_block::BlockError::Invalid(reason) => {
                    format!("The findings block wasn’t valid JSON ({reason}). Ask the Reviewer to send it again.")
                }
            };
            log::warn!("review {}: {detail}", workspace.id);
            let step = serde_json::json!({
                "icon": "list",
                "title": "Couldn’t read the review findings",
                "detail": detail,
            });
            state
                .store
                .append_event(thread_id, "tool", &step.to_string())?;
            emit_state_changed(app, &state);
            return Ok(());
        }
    };

    state.store.delete_findings_for_workspace(&workspace.id)?;
    for finding in findings {
        state.store.upsert_finding(&FindingRow {
            id: Uuid::new_v4().to_string(),
            workspace_id: workspace.id.clone(),
            severity: finding.severity,
            title: finding.title,
            file: finding.file,
            line: finding.line,
            explanation: finding.explanation,
            status: "open".into(),
            commit_sha: None,
            sent_to_thread_id: None,
        })?;
    }
    if branch_review {
        complete_branch_review(app, &workspace, thread_id)
    } else {
        complete_review(app, &workspace, thread_id).await
    }
}

fn branch_review_running(store: &Store, workspace_id: &str) -> bool {
    store
        .get_setting(&review_status_key(workspace_id))
        .ok()
        .flatten()
        .as_deref()
        == Some("running")
}

/// Review a regular workspace's own branch. A Reviewer thread (reused if one is open)
/// reads the changes against the base, and its findings fill the workspace's Findings
/// tab, from where they can be sent to the Lead. Returns the Reviewer's thread id.
pub async fn start_branch_review(
    app: &AppHandle,
    state: &AppState,
    workspace_id: &str,
) -> Result<String> {
    let workspace = state
        .store
        .workspace_by_id(workspace_id)?
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
    if workspace.kind.as_deref() == Some("review") {
        return Err(Error::Workspace(
            "this is a review workspace; its Reviewer already reviews the PR".into(),
        ));
    }
    let record = crate::git_workspace::resolve_record(state, workspace_id, &workspace).await?;

    let existing = state
        .store
        .snapshot()?
        .threads
        .into_iter()
        .find(|row| row.workspace_id == workspace_id && row.title == REVIEWER_TITLE);
    let thread = match existing {
        Some(row) if row.status == "running" => {
            return Err(Error::Workspace(
                "the Reviewer is still working on the last review".into(),
            ));
        }
        Some(row) => row,
        None => {
            let row = ThreadRow {
                id: Uuid::new_v4().to_string(),
                workspace_id: workspace_id.to_string(),
                title: REVIEWER_TITLE.into(),
                engine: default_engine(&state.store),
                session_id: None,
                status: "idle".into(),
                used_tokens: None,
                context_size: None,
                cost_usd: None,
                transcript_readonly: false,
            };
            state.store.upsert_thread(&row)?;
            row
        }
    };

    crate::provisioning::ensure_thread_engine(state, &thread.id, &thread.engine, workspace_id)
        .await?;
    let message = format!(
        "Review the changes on `{}` against `{}`. Flag bugs, risky changes and missing tests. Don’t edit anything; list findings for me to send back.",
        record.branch, record.base
    );
    crate::composer::persist_user_message(&state.store, &thread.id, &message)?;
    let prompt = format!("{message}\n\n{}", branch_reviewer_rubric(&record.base));
    state.engines.submit_prompt(&thread.id, prompt, false)?;
    state
        .store
        .set_setting(&review_status_key(workspace_id), "running")?;
    state.store.set_thread_status(&thread.id, "running")?;
    state
        .workspace
        .set_thread(&thread.id, workspace_id, ThreadActivity::Running)
        .await;
    state.store.set_workspace_status(workspace_id, "running")?;
    emit_state_changed(app, state);
    Ok(thread.id)
}

/// The turn-end handler has already replaced any earlier findings with this review's.
fn complete_branch_review(
    app: &AppHandle,
    workspace: &WorkspaceRow,
    thread_id: &str,
) -> Result<()> {
    let state = app.state::<AppState>();
    state
        .store
        .set_setting(&review_status_key(&workspace.id), "ready")?;
    let found = state
        .store
        .snapshot()?
        .findings
        .iter()
        .any(|row| row.workspace_id == workspace.id && row.status == "open");
    if !found {
        emit_toast(
            app,
            ToastRaisedPayload {
                tone: ToastTone::Ok,
                parts: vec![
                    ToastPart::Text {
                        value: "Review of ".into(),
                    },
                    ToastPart::Code {
                        value: workspace.branch.clone(),
                    },
                    ToastPart::Text {
                        value: " finished · no findings".into(),
                    },
                ],
                workspace_id: Some(workspace.id.clone()),
                thread_id: Some(thread_id.to_string()),
            },
        );
    }
    emit_state_changed(app, &state);
    Ok(())
}

async fn complete_review(app: &AppHandle, workspace: &WorkspaceRow, thread_id: &str) -> Result<()> {
    let state = app.state::<AppState>();
    let findings: Vec<FindingRow> = state
        .store
        .snapshot()?
        .findings
        .into_iter()
        .filter(|row| row.workspace_id == workspace.id)
        .collect();

    state
        .store
        .set_setting(&review_status_key(&workspace.id), "ready")?;

    let author = state
        .store
        .get_setting(&review_author_key(&workspace.id))?
        .unwrap_or_else(|| "the author".into());
    let pr_label = workspace
        .pr_number
        .map(|num| format!("#{num}"))
        .unwrap_or_else(|| "the PR".into());
    let card_summary = if findings.is_empty() {
        format!("Reviewed {pr_label} from {author}: no findings.")
    } else {
        format!(
            "Reviewed {pr_label} from {author}: {}. Choose which to send back for fixes in the Findings tab; nothing has been posted to GitHub yet.",
            count_findings(&findings)
        )
    };
    state.store.upsert_workspace(&WorkspaceRow {
        summary: Some(card_summary),
        summary_at: Some(chrono_timestamp()),
        ..workspace.clone()
    })?;
    let _ = state
        .workspace
        .set_provisioning_detail(&workspace.id, "Review ready", 0, None, None)
        .await;

    // With findings, the frontend toasts (and notifies) when they first appear.
    if findings.is_empty() {
        emit_toast(
            app,
            ToastRaisedPayload {
                tone: ToastTone::Ok,
                parts: vec![
                    ToastPart::Text {
                        value: "Review of ".into(),
                    },
                    ToastPart::Code { value: pr_label },
                    ToastPart::Text {
                        value: " finished · no findings".into(),
                    },
                ],
                workspace_id: Some(workspace.id.clone()),
                thread_id: Some(thread_id.to_string()),
            },
        );
    }
    emit_state_changed(app, &state);
    Ok(())
}

fn emit_state_changed(app: &AppHandle, state: &AppState) {
    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(app);
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
        .ok_or_else(|| Error::Github("GitHub is not signed in — run `gh auth login`".into()))?;

    let open_findings: Vec<FindingRow> = snapshot
        .findings
        .into_iter()
        .filter(|row| row.workspace_id == input.workspace_id && row.status == "open")
        .collect();

    let mut body_lines = Vec::new();
    let mut line_comments = Vec::new();
    for finding in &open_findings {
        if let (Some(path), Some(line)) = (finding.file.as_ref(), finding.line)
            && !path.is_empty()
            && line > 0
        {
            line_comments.push(ReviewLineComment {
                path: path.clone(),
                line,
                body: format!("**{}** — {}", finding.title, finding.explanation),
            });
            continue;
        }
        body_lines.push(format!(
            "- **{}** ({}) — {}",
            finding.title, finding.severity, finding.explanation
        ));
    }
    let body = if body_lines.is_empty() {
        "Review submitted from Cormux.".into()
    } else {
        format!("Review submitted from Cormux.\n\n{}", body_lines.join("\n"))
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
            thread_id: None,
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
    if let Some(url) = pr_html_url
        && let Some(rest) = url.strip_prefix("https://github.com/")
    {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() >= 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rubric_describes_the_findings_block() {
        let rubric = reviewer_rubric();
        assert!(rubric.contains("<cormux-findings>"));
        assert!(rubric.contains("</cormux-findings>"));
        assert!(!rubric.contains("report_finding"));
    }

    #[test]
    fn branch_rubric_covers_uncommitted_work_against_the_base() {
        let rubric = branch_reviewer_rubric("develop");
        assert!(rubric.contains("git diff develop...HEAD"));
        assert!(rubric.contains("git diff HEAD"));
        assert!(rubric.contains("<cormux-findings>"));
    }
}
