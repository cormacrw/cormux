use tokio::sync::broadcast::error::RecvError;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::feedback::Change;
use crate::feedback::{ToastPart, ToastRaisedPayload, ToastTone};
use crate::github::RepoSlug;
use crate::github::auth;
use crate::github::review::{ReviewLineComment, ReviewVerdict, SubmitPullRequestReviewInput};
use crate::model::{ThreadStatus, WorkspaceKind};
use crate::provisioning::{ProvisionPlan, WorkspaceSpec};
use crate::state::AppState;
use crate::store::Store;
use crate::store::types::{FindingRow, WorkspaceRow};
use crate::timeline::{TimelineStep, record_step};
use crate::workspace_git::Checkout;
use crate::workspaces::WorkspaceRecord;

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
    store.live_workspaces().ok()?.into_iter().find(|row| {
        row.kind == WorkspaceKind::PrReview
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

pub async fn create_review_workspace(
    state: &AppState,
    input: CreateReviewWorkspaceInput,
) -> Result<CreateReviewWorkspaceResult> {
    if let Some(existing) =
        find_existing_review_workspace(&state.store, &input.repo_id, input.pr_number)
    {
        // The PR may have been retargeted since.
        if existing.base != input.base {
            state.workspaces.set_base(&existing.id, &input.base).await?;
        }
        return Ok(CreateReviewWorkspaceResult {
            workspace_id: existing.id,
            created: false,
        });
    }

    let engine = default_engine(&state.store);
    let created = crate::provisioning::create(
        state,
        WorkspaceSpec {
            kind: WorkspaceKind::PrReview,
            repo_id: input.repo_id.clone(),
            name: format!("PR #{} Review", input.pr_number),
            branch: input.head.clone(),
            base: input.base.clone(),
            pr: Some((input.pr_number, input.pr_html_url.clone())),
            engine: engine.clone(),
            parent_thread_id: None,
        },
    )
    .await?;
    let workspace_id = created.workspace.id.clone();
    let thread_id = created.thread_id.clone();

    let user_message = user_review_message(
        input.pr_number,
        &input.title,
        &input.author,
        input.author_is_you,
    );
    // Left untyped on purpose: with no `type` the timeline skips this row, though the store's
    // message readers use it. A `MessageChunk` here would add a user bubble to every review thread.
    state.store.append_event(
        &thread_id,
        "message",
        &serde_json::json!({ "role": "user", "text": user_message }).to_string(),
    )?;

    record_step(
        &state.store,
        &thread_id,
        TimelineStep::PrCheckedOut {
            number: input.pr_number,
            head: input.head.clone(),
            base: input.base.clone(),
            files_changed: input.files_changed,
        },
    )?;

    let author_label = if input.author_is_you {
        "you".into()
    } else {
        format!("@{}", input.author)
    };
    state
        .store
        .set_setting(&review_author_key(&workspace_id), &author_label)?;

    state.out.changed(Change::THREADS);

    state.out.toast(ToastRaisedPayload {
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
    });

    let plan = review_plan(
        &created.workspace,
        &thread_id,
        &engine,
        &user_message,
        input.pr_number,
        author_label,
    );
    crate::provisioning::start(state, plan);

    Ok(CreateReviewWorkspaceResult {
        workspace_id,
        created: true,
    })
}

/// How a PR review comes up: the PR's head checked out, then the Reviewer reading it.
fn review_plan(
    workspace: &WorkspaceRecord,
    thread_id: &str,
    engine: &str,
    user_message: &str,
    pr_number: i64,
    author: String,
) -> ProvisionPlan {
    let pr_label = format!("#{pr_number}");
    let seed_label = pr_label.clone();
    ProvisionPlan {
        workspace_id: workspace.id.clone(),
        thread_id: thread_id.to_string(),
        engine: engine.to_string(),
        checkout: Checkout::PullRef(pr_number as u64),
        setup_activity: "Checking out the PR…".into(),
        first_prompt: Some(reviewer_engine_prompt(user_message).into()),
        running_activity: "Reading the diff…".into(),
        seed_summary: Some(Box::new(move |workspace: &WorkspaceRecord| {
            crate::summaries::review_started(&seed_label, &author, &workspace.base)
        })),
        started_toast: Some(vec![
            ToastPart::Text {
                value: "Reviewer started on ".into(),
            },
            ToastPart::Code { value: pr_label },
        ]),
    }
}

/// The plan again for a review whose setup failed.
pub fn resume_plan(state: &AppState, workspace: &WorkspaceRecord) -> Result<ProvisionPlan> {
    let row = state
        .store
        .workspace_by_id(&workspace.id)?
        .ok_or_else(|| Error::unknown_workspace(&workspace.id))?;
    let pr_number = row
        .pr_number
        .ok_or_else(|| Error::Workspace("Review workspace has no PR number".into()))?;
    let thread = state
        .store
        .own_thread(&workspace.id)?
        .ok_or_else(|| Error::Workspace(format!("no Reviewer in workspace {}", workspace.id)))?;
    let message = state
        .store
        .first_message_text(&thread.id)?
        .unwrap_or_default();
    let author = state
        .store
        .get_setting(&review_author_key(&workspace.id))?
        .unwrap_or_else(|| "the author".into());
    Ok(review_plan(
        workspace,
        &thread.id,
        &thread.engine,
        &message,
        pr_number,
        author,
    ))
}

/// Reads each Reviewer's findings when its turn ends, for the life of the app.
pub fn start(state: &AppState) {
    let state = state.clone();
    let mut rx = state.turn_end_notify.subscribe();
    tauri::async_runtime::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(thread_id) => {
                    if let Err(error) = on_reviewer_turn_end(&state, &thread_id).await {
                        log::warn!("reading review findings failed: {error}");
                    }
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
    });
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
pub async fn on_reviewer_turn_end(state: &AppState, thread_id: &str) -> Result<()> {
    let Some(thread) = state.store.thread_by_id(thread_id)? else {
        return Ok(());
    };
    let Some(workspace) = state.store.workspace_by_id(&thread.workspace_id)? else {
        return Ok(());
    };
    let branch_review = workspace.kind != WorkspaceKind::PrReview;
    let pending = if branch_review {
        branch_review_running(&state.store, &workspace.id)
    } else {
        !review_is_ready(&state.store, &workspace.id)
    };
    if workspace.archived_at.is_some() || !pending {
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
                    "The reply didn’t end with a findings block. Ask the agent to finish the review with its findings.".to_string()
                }
                crate::findings_block::BlockError::Invalid(reason) => {
                    format!("The findings block wasn’t valid JSON ({reason}). Ask the agent to send it again.")
                }
            };
            log::warn!("review {}: {detail}", workspace.id);
            record_step(
                &state.store,
                thread_id,
                TimelineStep::ReviewFindingsUnreadable { reason: detail },
            )?;
            state.out.changed(Change::THREADS);
            return Ok(());
        }
    };

    let rows: Vec<FindingRow> = findings
        .into_iter()
        .map(|finding| FindingRow {
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
        })
        .collect();
    state.store.replace_findings(&workspace.id, &rows)?;
    if branch_review {
        complete_branch_review(state, &workspace, thread_id)
    } else {
        complete_review(state, &workspace, thread_id).await
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

/// Review a regular workspace's own branch. The workspace's thread reads the changes against
/// the base, and its findings fill the Findings panel, from where they can be sent back to
/// the same thread to fix. Returns the thread's id.
pub async fn start_branch_review(state: &AppState, workspace_id: &str) -> Result<String> {
    let record = state.workspaces.get(workspace_id).await?;
    if record.kind == WorkspaceKind::PrReview {
        return Err(Error::Workspace(
            "this is a review workspace; its Reviewer already reviews the PR".into(),
        ));
    }

    let thread = state
        .store
        .threads_for_workspace(workspace_id)?
        .into_iter()
        .find(|row| !row.retired)
        .ok_or_else(|| Error::Workspace(format!("no thread in workspace {workspace_id}")))?;
    if thread.has_status(ThreadStatus::Running) {
        return Err(Error::Workspace(
            "the agent is still working; review once it's done".into(),
        ));
    }

    let message = format!(
        "Review the changes on `{}` against `{}`. Flag bugs, risky changes and missing tests. Don’t edit anything; list findings for me to send back.",
        record.branch, record.base
    );
    let prompt = format!("{message}\n\n{}", branch_reviewer_rubric(&record.base));
    crate::turn::start(state, &thread, &message, prompt).await?;
    state
        .store
        .set_setting(&review_status_key(workspace_id), "running")?;
    state.out.changed(Change::THREADS);
    Ok(thread.id)
}

/// The turn-end handler has already replaced any earlier findings with this review's.
fn complete_branch_review(
    state: &AppState,
    workspace: &WorkspaceRow,
    thread_id: &str,
) -> Result<()> {
    state
        .store
        .set_setting(&review_status_key(&workspace.id), "ready")?;
    let found = state
        .store
        .findings_for_workspace(&workspace.id)?
        .iter()
        .any(|row| row.status == "open");
    if !found {
        state.out.toast(ToastRaisedPayload {
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
        });
    }
    state.out.changed(Change::THREADS);
    Ok(())
}

async fn complete_review(
    state: &AppState,
    workspace: &WorkspaceRow,
    thread_id: &str,
) -> Result<()> {
    let findings: Vec<FindingRow> = state.store.findings_for_workspace(&workspace.id)?;

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
    let counted = (!findings.is_empty()).then(|| count_findings(&findings));
    let card_summary = crate::summaries::review_ready(&pr_label, &author, counted.as_deref());
    state
        .workspaces
        .set_summary(&workspace.id, &card_summary, crate::summaries::LOCAL_SOURCE)?;
    state
        .workspaces
        .set_activity(&workspace.id, "Review ready", 0, None)
        .await?;

    // With findings, the frontend toasts (and notifies) when they first appear.
    if findings.is_empty() {
        state.out.toast(ToastRaisedPayload {
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
        });
    }
    state.out.changed(Change::THREADS);
    Ok(())
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
    state: &AppState,
    input: SubmitWorkspaceReviewInput,
) -> Result<()> {
    if state
        .store
        .get_setting(&review_submitted_key(&input.workspace_id))?
        .as_deref()
        == Some("true")
    {
        return Err(Error::Workspace("Review already submitted".into()));
    }

    let workspace = state
        .store
        .workspace_by_id(&input.workspace_id)?
        .ok_or_else(|| Error::unknown_workspace(&input.workspace_id))?;
    if workspace.kind != WorkspaceKind::PrReview {
        return Err(Error::Workspace("Not a review workspace".into()));
    }
    let pr_number = workspace
        .pr_number
        .ok_or_else(|| Error::Workspace("Review workspace has no PR number".into()))?;

    let repo = state
        .store
        .repo_by_id(&workspace.repo_id)?
        .ok_or_else(|| Error::unknown_repo(&workspace.repo_id))?;
    let slug = match workspace.pr_html_url.as_deref().and_then(RepoSlug::parse) {
        Some(slug) => slug,
        None => RepoSlug::of_origin(&state.git, &crate::paths::expand_tilde(&repo.path)).await?,
    };
    let (owner, repo_name) = (slug.owner, slug.name);

    let token = auth::resolve_token(&state.github.gh).await?;

    let open_findings: Vec<FindingRow> = state
        .store
        .findings_for_workspace(&input.workspace_id)?
        .into_iter()
        .filter(|row| row.status == "open")
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

    let client = &state.github.rest;
    let mut review = SubmitPullRequestReviewInput {
        verdict,
        body,
        comments: line_comments,
    };
    let submitted = client
        .submit_pull_request_review(&token, &owner, &repo_name, pr_number, &review)
        .await;
    if let Err(error) = submitted {
        // GitHub rejects the whole review (422) when any inline comment sits outside the PR's
        // diff hunks, e.g. a finding on an unchanged line. Rather than block the submit on
        // one finding, post them all in the body instead.
        if review.comments.is_empty() || !error.to_string().contains("422") {
            return Err(error);
        }
        log::warn!("inline review comments rejected, moving them to the body: {error}");
        let moved: Vec<String> = review
            .comments
            .drain(..)
            .map(|comment| format!("- `{}:{}` {}", comment.path, comment.line, comment.body))
            .collect();
        review.body = format!("{}\n\n{}", review.body, moved.join("\n"));
        client
            .submit_pull_request_review(&token, &owner, &repo_name, pr_number, &review)
            .await?;
    }

    // Straight after the POST, before anything that can fail: a retry must not post twice.
    state
        .store
        .set_setting(&review_submitted_key(&input.workspace_id), "true")?;

    let workspace_threads = state.store.threads_for_workspace(&input.workspace_id)?;
    let reviewer_thread = state
        .store
        .own_thread(&input.workspace_id)?
        .ok_or_else(|| Error::Store("Reviewer thread missing".into()))?;

    for thread in &workspace_threads {
        crate::turn::mark_idle(state, thread).await?;
    }

    record_step(
        &state.store,
        &reviewer_thread.id,
        TimelineStep::ReviewSubmitted { number: pr_number },
    )?;

    state
        .workspaces
        .set_activity(&input.workspace_id, "Review submitted", 0, None)
        .await?;

    state.out.toast(ToastRaisedPayload {
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
    });

    state.out.changed(Change::THREADS);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::engines::EngineCommand;
    use crate::feedback::{Emitted, RecordingOutput};
    use crate::model::{ThreadRole, WorkspaceLifecycle};
    use crate::store::types::{RepoRecord, ThreadRow};

    /// A Lead workspace whose thread is in `status`.
    fn workspace(status: ThreadStatus) -> (AppState, Arc<RecordingOutput>) {
        let (state, recording) =
            AppState::recorded(crate::paths::DataDir::new(std::env::temp_dir()));
        state
            .store
            .upsert_repo(&RepoRecord {
                id: "r1".into(),
                path: "/tmp/app".into(),
                name: "app".into(),
                default_branch: Some("main".into()),
                setup_commands: String::new(),
                run_command: None,
                single_instance: false,
            })
            .unwrap();
        state
            .store
            .insert_workspace(
                &WorkspaceRow::new(
                    WorkspaceKind::Lead,
                    "w1",
                    "r1",
                    "Work",
                    "feat/x",
                    "main",
                    "/tmp/wt",
                )
                .with_status(WorkspaceLifecycle::Ready),
            )
            .unwrap();
        state
            .store
            .upsert_thread(
                &ThreadRow::new("t1", "w1", ThreadRole::Lead, "claude").with_status(status),
            )
            .unwrap();
        (state, recording)
    }

    /// A branch review in flight, its thread having just said `reply`.
    fn reviewing(reply: &str) -> (AppState, Arc<RecordingOutput>) {
        let (state, recording) = workspace(ThreadStatus::Idle);
        state
            .store
            .set_setting(&review_status_key("w1"), "running")
            .unwrap();
        let said = serde_json::json!({ "type": "messageChunk", "role": "agent", "text": reply });
        state
            .store
            .append_event("t1", "message", &said.to_string())
            .unwrap();
        (state, recording)
    }

    #[tokio::test]
    async fn a_clean_branch_review_toasts_then_refreshes() {
        let (state, recording) =
            reviewing("Looks fine.\n\n<cormux-findings>\n[]\n</cormux-findings>");

        on_reviewer_turn_end(&state, "t1").await.unwrap();

        assert_eq!(
            recording.toasts(),
            ["Review of feat/x finished · no findings"]
        );
        assert!(matches!(
            recording.emitted().as_slice(),
            [Emitted::Toast(_), Emitted::State(Change::THREADS)]
        ));
        assert!(review_is_ready(&state.store, "w1"));
    }

    #[tokio::test]
    async fn a_branch_review_finishes_when_its_turn_ends() {
        let (state, recording) = reviewing("Fine.\n\n<cormux-findings>\n[]\n</cormux-findings>");
        start(&state);

        state.turn_end_notify.send("t1".into()).unwrap();

        for _ in 0..250 {
            if review_is_ready(&state.store, "w1") {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert!(review_is_ready(&state.store, "w1"));
        assert_eq!(recording.toasts().len(), 1);
    }

    #[tokio::test]
    async fn a_finished_pr_review_saves_its_card_summary() {
        let (state, _recording) = workspace(ThreadStatus::Idle);
        state
            .store
            .insert_workspace(
                &WorkspaceRow::new(
                    WorkspaceKind::PrReview,
                    "pr",
                    "r1",
                    "PR #7 Review",
                    "fix/x",
                    "main",
                    "/tmp/pr",
                )
                .with_status(WorkspaceLifecycle::Running)
                .with_pr(7, None),
            )
            .unwrap();
        state
            .store
            .upsert_thread(
                &ThreadRow::new("rt", "pr", ThreadRole::Reviewer, "claude")
                    .with_status(ThreadStatus::Running),
            )
            .unwrap();
        state
            .store
            .set_setting(&review_author_key("pr"), "@ana")
            .unwrap();
        let said = serde_json::json!({
            "type": "messageChunk",
            "role": "agent",
            "text": "Fine.\n\n<cormux-findings>\n[]\n</cormux-findings>",
        });
        state
            .store
            .append_event("rt", "message", &said.to_string())
            .unwrap();
        state.workspaces.rename("pr", "Renamed").await.unwrap();

        on_reviewer_turn_end(&state, "rt").await.unwrap();

        let row = state.store.workspace_by_id("pr").unwrap().unwrap();
        assert_eq!(
            row.summary.as_deref(),
            Some("Reviewed #7 from @ana: no findings.")
        );
        assert_eq!(row.name, "Renamed");
        assert_eq!(
            state.workspaces.get("pr").await.unwrap().activity,
            "Review ready"
        );
    }

    #[tokio::test]
    async fn a_reply_with_no_findings_block_leaves_the_review_running() {
        let (state, recording) = reviewing("I got distracted.");

        on_reviewer_turn_end(&state, "t1").await.unwrap();

        assert!(recording.toasts().is_empty());
        assert_eq!(recording.state_changes(), [Change::THREADS]);
        assert!(branch_review_running(&state.store, "w1"));
    }

    fn prompts(engine: &mut tokio::sync::mpsc::UnboundedReceiver<EngineCommand>) -> Vec<String> {
        std::iter::from_fn(|| engine.try_recv().ok())
            .filter_map(|command| match command {
                EngineCommand::Prompt(prompt) => Some(prompt.text),
                _ => None,
            })
            .collect()
    }

    #[tokio::test]
    async fn a_branch_review_of_a_paused_thread_waits_for_its_resume() {
        let (state, recording) = workspace(ThreadStatus::Paused);
        let mut engine = state.engines.fake_thread("t1");

        let thread_id = start_branch_review(&state, "w1").await.unwrap();

        assert_eq!(thread_id, "t1");
        let thread = state.store.thread_by_id("t1").unwrap().unwrap();
        assert!(thread.has_status(ThreadStatus::Paused));
        assert!(prompts(&mut engine).is_empty());
        assert!(recording.state_changes().contains(&Change::THREADS));

        crate::turn::resume(&state, "t1").await.unwrap();
        assert!(prompts(&mut engine)[0].contains("<cormux-findings>"));
    }

    #[tokio::test]
    async fn a_branch_review_the_engine_refuses_never_starts() {
        let (state, _recording) = workspace(ThreadStatus::Idle);
        let _engine = state.engines.fake_thread("t1");
        state.engines.refuse_sends("t1");

        assert!(start_branch_review(&state, "w1").await.is_err());
        let thread = state.store.thread_by_id("t1").unwrap().unwrap();
        assert!(thread.has_status(ThreadStatus::Idle));
        assert!(!review_is_ready(&state.store, "w1"));
        assert_eq!(
            state.store.get_setting(&review_status_key("w1")).unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn a_branch_review_is_refused_while_the_thread_is_running() {
        let (state, _recording) = workspace(ThreadStatus::Running);
        let mut engine = state.engines.fake_thread("t1");

        assert!(start_branch_review(&state, "w1").await.is_err());
        assert!(prompts(&mut engine).is_empty());
    }

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
