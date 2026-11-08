mod findings_block;
mod store;

use crate::bus::DomainEvent;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::feedback::{ToastPart, ToastRaisedPayload, ToastTone};
use crate::findings::FindingRow;
use crate::github::RepoSlug;
use crate::github::auth;
use crate::github::review::{
    ReviewLineComment, ReviewPosted, ReviewVerdict, SubmitPullRequestReviewInput,
};
use crate::github::types::PullRequestPayload;
use crate::model::WorkspaceKind;
use crate::provisioning::{ProvisionPlan, WorkspaceSpec};
use crate::state::AppState;
use crate::store::Store;
use crate::store::types::WorkspaceRow;
use crate::timeline::TimelineStep;
use crate::transcript::TranscriptEvent;
use crate::workspace_git::Checkout;
use crate::workspaces::{WorkspaceActivity, WorkspaceRecord};

pub(crate) use store::live as live_in;
pub use store::{ReviewRow, ReviewStatus};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateReviewWorkspaceInput {
    pub repo_id: String,
    pub pr_number: i64,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateReviewWorkspaceResult {
    pub workspace_id: String,
    pub created: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SubmitWorkspaceReviewInput {
    pub workspace_id: String,
    pub verdict: ReviewVerdict,
}

fn user_review_message(pr: &PullRequestPayload) -> String {
    let by = if pr.authored_by_viewer() {
        "you".to_string()
    } else {
        format!("@{}", pr.author)
    };
    format!(
        "Review #{} “{}” by {by}. Flag bugs, risky changes and missing tests. Don’t push commits; draft comments for me to approve.",
        pr.num, pr.title
    )
}

/// Sent to the engine after the user's message but never shown in the thread. The
/// findings block it asks for is parsed at the end of the turn and hidden in the UI.
fn reviewer_rubric() -> String {
    rubric_with_intro(
        "You are the Reviewer for this pull request. Its branch is checked out in your working directory; \
         use `gh pr diff` or git against the base branch to see what changed, and read the surrounding code. \
         Look for bugs, risky changes and missing tests. Do not edit files, push commits or post to GitHub.",
        "line number in the PR's version of the file, or null.",
    )
}

fn branch_reviewer_rubric(base: &str) -> String {
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
        open = findings_block::OPEN_TAG,
        close = findings_block::CLOSE_TAG,
    )
}

fn reviewer_engine_prompt(user_message: &str) -> String {
    format!("{user_message}\n\n{}", reviewer_rubric())
}

fn find_existing_review_workspace(
    store: &Store,
    repo_id: &str,
    pr_number: i64,
) -> Result<Option<WorkspaceRow>> {
    Ok(store.live_workspaces()?.into_iter().find(|row| {
        row.kind == WorkspaceKind::PrReview
            && row.repo_id == repo_id
            && row.pr_number == Some(pr_number)
    }))
}

/// A review workspace for the repo's pull request `pr_number`, as the last PR sync saw it, or
/// the one already open for it: one per PR, the rule's only owner.
pub async fn create_review_workspace(
    state: &AppState,
    input: CreateReviewWorkspaceInput,
) -> Result<CreateReviewWorkspaceResult> {
    let pr = state
        .store
        .cached_pull_request(&input.repo_id, input.pr_number)?
        .ok_or_else(|| {
            Error::Invalid(format!(
                "pull request #{} of repo {} isn't in the PR list",
                input.pr_number, input.repo_id
            ))
        })?;
    if let Some(existing) =
        find_existing_review_workspace(&state.store, &input.repo_id, input.pr_number)?
    {
        // The PR may have been retargeted since.
        if existing.base != pr.base {
            state.workspaces.set_base(&existing.id, &pr.base).await?;
            state.out.changed();
        }
        return Ok(CreateReviewWorkspaceResult {
            workspace_id: existing.id,
            created: false,
        });
    }

    let engine = crate::settings::Settings::load(&state.store).default_engine_name();
    let created = crate::provisioning::create(
        state,
        WorkspaceSpec {
            kind: WorkspaceKind::PrReview,
            repo_id: input.repo_id.clone(),
            name: format!("PR #{} Review", pr.num),
            branch: pr.head.clone(),
            base: pr.base.clone(),
            pr: Some((
                pr.num,
                (!pr.html_url.is_empty()).then(|| pr.html_url.clone()),
            )),
            engine: engine.clone(),
            parent_thread_id: None,
            picks: Default::default(),
        },
    )
    .await?;
    let workspace_id = created.workspace.id.clone();
    let thread_id = created.thread_id.clone();

    let user_message = user_review_message(&pr);
    state
        .transcript
        .append(&thread_id, TranscriptEvent::briefing(&user_message))?;
    state.transcript.record_step(
        &thread_id,
        TimelineStep::PrCheckedOut {
            number: pr.num,
            head: pr.head.clone(),
            base: pr.base.clone(),
            files_changed: pr.files,
        },
    )?;

    state
        .store
        .set_review_status(&workspace_id, ReviewStatus::Running)?;

    state.out.changed();

    state.out.toast(ToastRaisedPayload {
        tone: ToastTone::Ok,
        parts: vec![
            ToastPart::Text {
                value: "Opened a review workspace for ".into(),
            },
            ToastPart::Code {
                value: format!("#{}", pr.num),
            },
        ],
        workspace_id: Some(workspace_id.clone()),
        thread_id: Some(thread_id.clone()),
        ..Default::default()
    });

    let plan = review_plan(&created.workspace, &thread_id, &user_message, pr.num);
    crate::provisioning::start(state, plan);

    Ok(CreateReviewWorkspaceResult {
        workspace_id,
        created: true,
    })
}

fn review_plan(
    workspace: &WorkspaceRecord,
    thread_id: &str,
    user_message: &str,
    pr_number: i64,
) -> ProvisionPlan {
    let pr_label = format!("#{pr_number}");
    ProvisionPlan {
        workspace_id: workspace.id.clone(),
        thread_id: thread_id.to_string(),
        checkout: Checkout::PullRef(pr_number as u64),
        first_prompt: Some(reviewer_engine_prompt(user_message).into()),
        intro: None,
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
        .transcript
        .first_message(&thread.id)?
        .unwrap_or_default();
    Ok(review_plan(workspace, &thread.id, &message, pr_number))
}

/// Reads each Reviewer's findings when its turn ends, for the life of the app.
pub fn start(state: &AppState) {
    let state = state.clone();
    state.bus.clone().listen(move |event| {
        let state = state.clone();
        async move {
            if let DomainEvent::TurnEnded { thread_id } = event
                && let Err(error) = on_reviewer_turn_end(&state, &thread_id).await
            {
                log::warn!("reading review findings failed: {error}");
            }
        }
    });
}

fn review_status(store: &Store, workspace_id: &str) -> Result<Option<ReviewStatus>> {
    Ok(store.review(workspace_id)?.and_then(|review| review.status))
}

/// Runs when any thread's turn ends. A missing or malformed findings block leaves the review
/// running, so the next turn's reply is read again.
async fn on_reviewer_turn_end(state: &AppState, thread_id: &str) -> Result<()> {
    let Some(thread) = state.store.thread_by_id(thread_id)? else {
        return Ok(());
    };
    let Some(workspace) = state.store.workspace_by_id(&thread.workspace_id)? else {
        return Ok(());
    };
    let pending = review_status(&state.store, &workspace.id)? == Some(ReviewStatus::Running);
    if workspace.archived_at.is_some() || !pending {
        return Ok(());
    }

    let reply = state.transcript.last_agent_reply(thread_id)?;
    let findings = match findings_block::parse_findings_block(&reply, &workspace.worktree_path) {
        Ok(findings) => findings,
        Err(error) => {
            let detail = match error {
                findings_block::BlockError::Missing => {
                    "The reply didn’t end with a findings block. Ask the agent to finish the review with its findings.".to_string()
                }
                findings_block::BlockError::Invalid(reason) => {
                    format!("The findings block wasn’t valid JSON ({reason}). Ask the agent to send it again.")
                }
            };
            log::warn!("review {}: {detail}", workspace.id);
            state.transcript.record_step(
                thread_id,
                TimelineStep::ReviewFindingsUnreadable { reason: detail },
            )?;
            state.out.changed();
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
            status: crate::model::FindingStatus::Open,
            commit_sha: None,
            sent_to_thread_id: None,
        })
        .collect();
    state.store.replace_findings(&workspace.id, &rows)?;
    complete_review(state, &workspace, thread_id, rows.len()).await
}

/// The workspace's own thread reviews its branch against the base, and its findings fill the
/// Findings panel, from where they can be sent back to the same thread to fix.
pub async fn start_branch_review(state: &AppState, workspace_id: &str) -> Result<String> {
    let record = state.workspaces.get(workspace_id).await?;
    if record.kind == WorkspaceKind::PrReview {
        return Err(Error::Workspace(
            "this is a review workspace; its Reviewer already reviews the PR".into(),
        ));
    }

    let thread = state
        .store
        .own_thread(workspace_id)?
        .ok_or_else(|| Error::Workspace(format!("no Lead in workspace {workspace_id}")))?;
    if thread.status.is_working() {
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
        .set_review_status(workspace_id, ReviewStatus::Running)?;
    state.out.changed();
    Ok(thread.id)
}

/// The turn-end handler has already replaced any earlier findings with this review's. The one
/// place a review is said to have finished: a toast that opens the findings, also said as a
/// desktop notification when the user wants one.
async fn complete_review(
    state: &AppState,
    workspace: &WorkspaceRow,
    thread_id: &str,
    found: usize,
) -> Result<()> {
    state
        .store
        .set_review_status(&workspace.id, ReviewStatus::Ready)?;
    let reviewed = match workspace.kind {
        WorkspaceKind::PrReview => {
            state
                .workspaces
                .set_activity(&workspace.id, WorkspaceActivity::ReviewReady)
                .await?;
            workspace
                .pr_number
                .map(|num| format!("#{num}"))
                .unwrap_or_else(|| "the PR".into())
        }
        WorkspaceKind::Lead => workspace.branch.clone(),
    };
    if found > 0 {
        state.transcript.record_step(
            thread_id,
            TimelineStep::ReviewFindingsReady {
                count: found as u32,
            },
        )?;
    }
    let parts = review_finished_parts(&reviewed, found);
    let notify = crate::settings::Settings::load(&state.store).notify_review_finished;
    state.out.toast(ToastRaisedPayload {
        tone: ToastTone::Ok,
        notification: notify.then(|| ToastPart::plain(&parts)),
        parts,
        workspace_id: Some(workspace.id.clone()),
        thread_id: Some(thread_id.to_string()),
        opens_findings: found > 0,
    });
    state.out.changed();
    Ok(())
}

/// `Review of #12 finished · 3 findings`, with what was reviewed and the count set as code.
fn review_finished_parts(reviewed: &str, found: usize) -> Vec<ToastPart> {
    let mut parts = vec![
        ToastPart::text("Review of "),
        ToastPart::code(reviewed),
        ToastPart::text(" finished · "),
    ];
    match found {
        0 => parts.push(ToastPart::text("no findings")),
        1 => parts.extend([ToastPart::code("1"), ToastPart::text(" finding")]),
        n => parts.extend([ToastPart::code(n.to_string()), ToastPart::text(" findings")]),
    }
    parts
}

pub async fn submit_workspace_review(
    state: &AppState,
    input: SubmitWorkspaceReviewInput,
) -> Result<()> {
    if state
        .store
        .review(&input.workspace_id)?
        .is_some_and(|review| review.submitted)
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

    let repo = state.store.repo(&workspace.repo_id)?;
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
        .filter(|row| row.status == crate::model::FindingStatus::Open)
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

    let client = &state.github.rest;
    let mut review = SubmitPullRequestReviewInput {
        verdict: input.verdict,
        body,
        comments: line_comments,
    };
    let posted = client
        .submit_pull_request_review(&token, &owner, &repo_name, pr_number, &review)
        .await?;
    // Rather than block the submit on one finding, post them all in the body instead.
    if let ReviewPosted::CommentsRejected(error) = posted {
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
    state.store.mark_review_submitted(&input.workspace_id)?;

    let reviewer_thread = state
        .store
        .own_thread(&input.workspace_id)?
        .ok_or_else(|| Error::Store("Reviewer thread missing".into()))?;

    state.transcript.record_step(
        &reviewer_thread.id,
        TimelineStep::ReviewSubmitted { number: pr_number },
    )?;

    state
        .workspaces
        .set_activity(&input.workspace_id, WorkspaceActivity::ReviewSubmitted)
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
        ..Default::default()
    });

    state.out.changed();
    Ok(())
}

pub mod commands {
    use tauri::State;

    use crate::error::Result;
    use crate::state::AppState;

    use super::{
        CreateReviewWorkspaceInput, CreateReviewWorkspaceResult, SubmitWorkspaceReviewInput,
    };

    #[tauri::command]
    #[specta::specta]
    pub async fn create_review_workspace(
        input: CreateReviewWorkspaceInput,
        state: State<'_, AppState>,
    ) -> Result<CreateReviewWorkspaceResult> {
        super::create_review_workspace(&state, input).await
    }

    #[tauri::command]
    #[specta::specta]
    pub async fn submit_workspace_review(
        input: SubmitWorkspaceReviewInput,
        state: State<'_, AppState>,
    ) -> Result<()> {
        super::submit_workspace_review(&state, input).await
    }

    /// Start a Reviewer on a regular workspace's own branch. Returns its thread id.
    #[tauri::command]
    #[specta::specta]
    pub async fn start_branch_review(
        workspace_id: String,
        state: State<'_, AppState>,
    ) -> Result<String> {
        super::start_branch_review(&state, &workspace_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::engines::EngineCommand;
    use crate::feedback::RecordingOutput;
    use crate::model::{ThreadRole, ThreadStatus, WorkspaceLifecycle};
    use crate::store::types::{RepoRecord, ThreadRow};
    use crate::thread_runtime::fake::FakeEngines;

    fn review_is_ready(store: &Store, workspace_id: &str) -> bool {
        review_status(store, workspace_id).unwrap() == Some(ReviewStatus::Ready)
    }

    fn branch_review_running(store: &Store, workspace_id: &str) -> bool {
        review_status(store, workspace_id).unwrap() == Some(ReviewStatus::Running)
    }

    /// A Lead workspace whose thread is in `status`.
    fn workspace(status: ThreadStatus) -> (AppState, Arc<RecordingOutput>, FakeEngines) {
        let (state, recording, fake) =
            AppState::faked(crate::paths::DataDir::new(std::env::temp_dir()));
        state
            .store
            .upsert_repo(&RepoRecord {
                path: "/tmp/app".into(),
                name: "app".into(),
                ..crate::store::fixtures::repo("r1")
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
                    std::env::temp_dir().display().to_string(),
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
        (state, recording, fake)
    }

    /// A branch review in flight, its thread having just said `reply`.
    fn reviewing(reply: &str) -> (AppState, Arc<RecordingOutput>) {
        let (state, recording, _) = workspace(ThreadStatus::Idle);
        state
            .store
            .set_review_status("w1", ReviewStatus::Running)
            .unwrap();
        state.transcript.append("t1", agent_said(reply)).unwrap();
        (state, recording)
    }

    #[tokio::test]
    async fn a_clean_branch_review_toasts_and_refreshes() {
        let (state, recording) =
            reviewing("Looks fine.\n\n<cormux-findings>\n[]\n</cormux-findings>");

        on_reviewer_turn_end(&state, "t1").await.unwrap();

        assert_eq!(
            recording.toasts(),
            ["Review of feat/x finished · no findings"]
        );
        assert!(!recording.state_changes().is_empty());
        assert!(review_is_ready(&state.store, "w1"));
    }

    #[tokio::test]
    async fn findings_get_their_card_in_the_thread_that_ran_the_review_only() {
        let (state, recording) = reviewing(
            "One thing.\n\n<cormux-findings>\n[{\"severity\": \"blocking\", \"title\": \"Leak\", \"file\": null, \"line\": null, \"explanation\": \"x\"}]\n</cormux-findings>",
        );
        state
            .store
            .upsert_thread(&ThreadRow::new("other", "w1", ThreadRole::Lead, "claude"))
            .unwrap();

        on_reviewer_turn_end(&state, "t1").await.unwrap();

        let card = |thread: &str| {
            state
                .store
                .thread_events_after(thread, 0, 50)
                .unwrap()
                .iter()
                .any(|(_, _, payload)| payload.contains("\"reviewFindingsReady\""))
        };
        assert!(card("t1"));
        assert!(!card("other"));
        assert_eq!(
            recording.toasts(),
            ["Review of feat/x finished · 1 finding"]
        );
    }

    fn toast_payloads(recording: &RecordingOutput) -> Vec<ToastRaisedPayload> {
        recording
            .emitted()
            .into_iter()
            .filter_map(|event| match event {
                crate::feedback::Emitted::Toast(payload) => Some(payload),
                _ => None,
            })
            .collect()
    }

    /// Bug: the webview said a review finished when the open findings went from none to some,
    /// so restoring dismissed findings raised one, and a re-review with findings still open
    /// raised none. The core says it once per finished review, findings or not.
    #[tokio::test]
    async fn every_finished_review_says_so_once_and_opens_its_findings() {
        let block = "Two.\n\n<cormux-findings>\n[{\"severity\": \"blocking\", \"title\": \"A\", \"file\": null, \"line\": null, \"explanation\": \"x\"}, {\"severity\": \"nit\", \"title\": \"B\", \"file\": null, \"line\": null, \"explanation\": \"y\"}]\n</cormux-findings>";
        let (state, recording) = reviewing(block);

        on_reviewer_turn_end(&state, "t1").await.unwrap();
        state
            .store
            .set_review_status("w1", ReviewStatus::Running)
            .unwrap();
        on_reviewer_turn_end(&state, "t1").await.unwrap();

        let toasts = toast_payloads(&recording);
        assert_eq!(toasts.len(), 2, "the re-review says so too");
        for toast in &toasts {
            assert!(toast.opens_findings);
            assert_eq!(toast.workspace_id.as_deref(), Some("w1"));
            assert_eq!(
                toast.notification.as_deref(),
                Some("Review of feat/x finished · 2 findings")
            );
        }
    }

    #[tokio::test]
    async fn a_finished_review_is_a_notification_only_when_the_user_wants_one() {
        let (state, recording) =
            reviewing("Looks fine.\n\n<cormux-findings>\n[]\n</cormux-findings>");
        crate::settings::Settings {
            notify_review_finished: false,
            ..Default::default()
        }
        .save(&state.store);

        on_reviewer_turn_end(&state, "t1").await.unwrap();

        let toasts = toast_payloads(&recording);
        assert_eq!(toasts.len(), 1);
        assert!(!toasts[0].opens_findings, "nothing to open");
        assert_eq!(toasts[0].notification, None);
    }

    #[tokio::test]
    async fn a_branch_review_finishes_when_its_turn_ends() {
        let (state, recording) = reviewing("Fine.\n\n<cormux-findings>\n[]\n</cormux-findings>");
        start(&state);

        state.bus.publish(DomainEvent::TurnEnded {
            thread_id: "t1".into(),
        });

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
    async fn a_finished_pr_review_reads_as_ready_under_its_own_name() {
        let (state, _recording, _) = workspace(ThreadStatus::Idle);
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
            .set_review_status("pr", ReviewStatus::Running)
            .unwrap();
        let said = agent_said("Fine.\n\n<cormux-findings>\n[]\n</cormux-findings>");
        state.transcript.append("rt", said).unwrap();
        state.workspaces.rename("pr", "Renamed").await.unwrap();

        on_reviewer_turn_end(&state, "rt").await.unwrap();

        let row = state.store.workspace_by_id("pr").unwrap().unwrap();
        assert_eq!(row.name, "Renamed");
        assert_eq!(
            state.workspaces.get("pr").await.unwrap().activity,
            WorkspaceActivity::ReviewReady
        );
    }

    #[tokio::test]
    async fn a_reply_with_no_findings_block_leaves_the_review_running() {
        let (state, recording) = reviewing("I got distracted.");

        on_reviewer_turn_end(&state, "t1").await.unwrap();

        assert!(recording.toasts().is_empty());
        assert!(branch_review_running(&state.store, "w1"));
    }

    fn agent_said(text: &str) -> crate::engines::AgentEvent {
        crate::engines::AgentEvent::MessageChunk {
            role: crate::engines::MessageRole::Agent,
            text: text.into(),
        }
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
        let (state, recording, fake) = workspace(ThreadStatus::Paused);
        let mut engine = fake.engine("t1");
        state.engines.ensure("t1", "w1").await.unwrap();

        let thread_id = start_branch_review(&state, "w1").await.unwrap();

        assert_eq!(thread_id, "t1");
        let thread = state.store.thread_by_id("t1").unwrap().unwrap();
        assert_eq!(thread.status, ThreadStatus::Paused);
        assert!(prompts(&mut engine).is_empty());
        assert!(!recording.state_changes().is_empty());

        crate::turn::resume(&state, "t1").await.unwrap();
        assert!(prompts(&mut engine)[0].contains("<cormux-findings>"));
    }

    #[tokio::test]
    async fn a_branch_review_the_engine_refuses_never_starts() {
        let (state, _recording, fake) = workspace(ThreadStatus::Idle);
        fake.refuse_sends("t1");

        assert!(start_branch_review(&state, "w1").await.is_err());
        let thread = state.store.thread_by_id("t1").unwrap().unwrap();
        assert_eq!(thread.status, ThreadStatus::Idle);
        assert!(!review_is_ready(&state.store, "w1"));
        assert_eq!(state.store.review("w1").unwrap(), None);
    }

    /// Bug: only `running` was refused, so a review could start on a thread still being set
    /// up, and setup's first turn then relaunched the engine under it.
    #[tokio::test]
    async fn a_branch_review_is_refused_while_the_thread_is_working() {
        for status in ThreadStatus::WORKING {
            let (state, _recording, fake) = workspace(status);
            let mut engine = fake.engine("t1");

            assert!(
                start_branch_review(&state, "w1").await.is_err(),
                "{status:?}"
            );
            assert!(prompts(&mut engine).is_empty());
        }
    }

    #[test]
    fn the_format_each_rubric_shows_is_one_the_parser_reads() {
        for rubric in [reviewer_rubric(), branch_reviewer_rubric("develop")] {
            let example = findings_block::parse_findings_block(&rubric, "/repo").unwrap();
            assert_eq!(example.len(), 1, "{rubric}");
            assert_eq!(example[0].severity, "blocking");
            assert_eq!(example[0].line, Some(48));
        }
    }

    /// Bug: the webview reopened an existing review itself, so this never ran and a
    /// retargeted PR's review kept comparing against the old base.
    #[tokio::test]
    async fn reopening_a_review_follows_a_retargeted_pr() {
        let (state, recording, _fake) = workspace(ThreadStatus::Idle);
        let review = WorkspaceRow::new(
            WorkspaceKind::PrReview,
            "w2",
            "r1",
            "PR #7 Review",
            "feat/pr",
            "main",
            std::env::temp_dir()
                .join("cormux-review-w2")
                .display()
                .to_string(),
        )
        .with_pr(7, None);
        state.store.insert_workspace(&review).unwrap();
        state
            .store
            .upsert_pr(&crate::github::types::PullRequestPayload {
                num: 7,
                title: "Retargeted".into(),
                author: "ana".into(),
                head: "feat/pr".into(),
                base: "release".into(),
                updated_at: "2026-01-01T00:00:00Z".into(),
                checks: crate::github::types::PrChecksState::Pass,
                review: crate::github::types::PrReviewState::Required,
                files: 1,
                is_draft: false,
                html_url: "https://github.com/o/r/pull/7".into(),
                repo_full_name: "o/r".into(),
                repo_id: Some("r1".into()),
            })
            .unwrap();

        let result = create_review_workspace(
            &state,
            CreateReviewWorkspaceInput {
                repo_id: "r1".into(),
                pr_number: 7,
            },
        )
        .await
        .unwrap();

        assert_eq!(
            (result.workspace_id.as_str(), result.created),
            ("w2", false)
        );
        assert_eq!(
            state.store.workspace_by_id("w2").unwrap().unwrap().base,
            "release"
        );
        assert_eq!(state.workspaces.get("w2").await.unwrap().base, "release");
        assert!(!recording.state_changes().is_empty());
    }
}
