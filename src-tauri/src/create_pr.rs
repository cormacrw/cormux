use std::path::Path;

use tauri::AppHandle;
use tauri_specta::Event;

use crate::engines::{AgentEvent, ToolCallStatus, ToolKind};
use crate::error::{Error, Result};
use crate::feedback::emit_toast;
use crate::git_workspace::{lead_thread_id, resolve_record};
use crate::github::auth;
use crate::github::create::{CreatePullRequestInput, RestGithubClient};
use crate::github::types::{PrChecksState, PrRelationship, PrReviewState, PullRequestPayload};
use crate::ipc::events::StateChanged;
use crate::ipc::types::{
    CreateWorkspacePullRequestInput, CreateWorkspacePullRequestResult, DraftPrWhyResult,
    StateChangeKind, ToastPart, ToastRaisedPayload, ToastTone,
};
use crate::pr_draft::{draft_why, extract_goal_from_events};
use crate::state::AppState;
use crate::store::types::{PrRow, WorkspaceRow};

pub async fn draft_pr_why(state: &AppState, workspace_id: &str) -> Result<DraftPrWhyResult> {
    let snapshot = state.store.snapshot()?;
    let workspace = snapshot
        .workspaces
        .iter()
        .find(|row| row.id == workspace_id)
        .cloned()
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
    if workspace.pr_number.is_some() {
        return Err(Error::Workspace("pull request already opened".into()));
    }

    let record = resolve_record(state, workspace_id, &workspace).await?;
    let worktree = Path::new(&record.worktree_path);
    let base = crate::stack::pr_base(state, worktree, &record.branch, &record.base).await;
    let diff = pr_diff(state, workspace_id, worktree, &base, &record.base).await;

    let lead_id = lead_thread_id(state, workspace_id)?;
    let events: Vec<_> = snapshot
        .timeline
        .iter()
        .filter(|row| row.thread_id == lead_id)
        .cloned()
        .collect();
    let goal = extract_goal_from_events(&events);
    let transcript = state.store.transcript_summary(&lead_id, 24)?;

    let (text, from_llm) = draft_why(
        &state.llm,
        workspace_id,
        &workspace,
        &base,
        &goal,
        &transcript,
        &diff,
    )
    .await?;

    Ok(DraftPrWhyResult {
        workspace_id: workspace_id.to_string(),
        text,
        from_llm,
    })
}

pub async fn create_workspace_pull_request(
    app: &AppHandle,
    state: &AppState,
    input: CreateWorkspacePullRequestInput,
) -> Result<CreateWorkspacePullRequestResult> {
    let why = input.why.trim();
    if why.is_empty() {
        return Err(Error::Workspace(
            "Add a reason so reviewers know what this fixes.".into(),
        ));
    }

    let snapshot = state.store.snapshot()?;
    let workspace = snapshot
        .workspaces
        .iter()
        .find(|row| row.id == input.workspace_id)
        .cloned()
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {}", input.workspace_id)))?;

    if workspace.pr_number.is_some() {
        return Err(Error::Workspace("pull request already opened".into()));
    }

    let record = resolve_record(state, &input.workspace_id, &workspace).await?;
    let worktree = Path::new(&record.worktree_path);

    let porcelain = state.git.status_porcelain(worktree).await?;
    if !porcelain.trim().is_empty() {
        return Err(Error::Git(
            "commit or stash uncommitted changes before opening a pull request".into(),
        ));
    }

    // A stacked branch's PR targets the branch below it, which GitHub needs on the remote.
    let pr_base = crate::stack::pr_base(state, worktree, &record.branch, &record.base).await;
    if pr_base != record.base
        && !state
            .git
            .remote_branch_exists(worktree, &pr_base)
            .await
            .unwrap_or(false)
    {
        return Err(Error::Git(format!(
            "{} is stacked on {pr_base}, which isn't on GitHub yet. Push the stack from the Stack tab, then try again.",
            record.branch
        )));
    }

    let token = auth::resolve_token(&state.shell_env)
        .await
        .ok_or_else(|| Error::Github("GitHub is not signed in — add a token in Settings".into()))?;

    state
        .git
        .push(worktree, &record.branch)
        .await
        .map_err(|error| match error {
            Error::Git(message) => Error::Git(format!(
                "push failed: {message}. Fix the remote or credentials, then try again."
            )),
            other => other,
        })?;

    refresh_git_stats(
        state,
        &input.workspace_id,
        worktree,
        &record.base,
        &record.branch,
    )
    .await;

    let origin = state
        .git
        .remote_origin_url(worktree)
        .await?
        .ok_or_else(|| Error::Git("no origin remote configured for this repo".into()))?;
    let slug = crate::github::parse_origin_url(&origin)
        .ok_or_else(|| Error::Git(format!("could not parse origin URL {origin}")))?;
    let (owner, repo) = slug
        .split_once('/')
        .ok_or_else(|| Error::Git(format!("invalid origin slug {slug}")))?;

    let diff = pr_diff(state, &input.workspace_id, worktree, &pr_base, &record.base).await;

    let title = input
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(workspace.name.as_str())
        .to_string();

    let body = build_pr_body(
        why,
        &diff,
        &workspace,
        input.include_what_changed,
        input.include_how_tested,
    );

    let client = RestGithubClient::new();
    let created = client
        .create_pull_request(
            &token,
            owner,
            repo,
            &CreatePullRequestInput {
                title: title.clone(),
                body,
                head: record.branch.clone(),
                base: pr_base.clone(),
                draft: input.draft,
            },
        )
        .await?;

    state.store.set_workspace_pr(
        &input.workspace_id,
        Some(created.number),
        Some(&created.html_url),
    )?;

    let (additions, deletions, files) = diff_totals(&diff);
    let updated_at = iso_timestamp_now();
    let review = if input.draft {
        PrReviewState::Draft
    } else {
        PrReviewState::Required
    };
    let payload = PullRequestPayload {
        num: created.number,
        title: title.clone(),
        author: "you".into(),
        rel: PrRelationship::Author,
        head: record.branch.clone(),
        base: pr_base.clone(),
        updated_at: updated_at.clone(),
        checks: PrChecksState::Running,
        failing: None,
        review,
        additions: additions as i64,
        deletions: deletions as i64,
        files: files as i64,
        is_draft: input.draft,
        html_url: created.html_url.clone(),
        repo_full_name: slug.clone(),
        repo_id: Some(workspace.repo_id.clone()),
    };
    let payload_json =
        serde_json::to_string(&payload).map_err(|error| Error::Store(error.to_string()))?;
    state.store.upsert_pr(&PrRow {
        id: payload.cache_id(),
        repo_id: Some(workspace.repo_id.clone()),
        number: created.number,
        title: title.clone(),
        payload: payload_json,
    })?;

    let lead_id = lead_thread_id(state, &input.workspace_id)?;
    append_opened_pr_step(
        &state.store,
        &lead_id,
        created.number,
        &record.branch,
        &pr_base,
    )?;

    emit_toast(
        app,
        ToastRaisedPayload {
            tone: ToastTone::Ok,
            parts: vec![
                ToastPart::Text {
                    value: format!("Opened PR #{} for ", created.number),
                },
                ToastPart::Code {
                    value: record.branch.clone(),
                },
            ],
            workspace_id: Some(input.workspace_id.clone()),
        },
    );

    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::PrSync,
    }
    .emit(app);
    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(app);

    Ok(CreateWorkspacePullRequestResult {
        workspace_id: input.workspace_id,
        number: created.number,
        html_url: created.html_url,
        title,
    })
}

/// The changes the PR will show. A stacked PR is measured against the branch below it;
/// otherwise this is the Changes panel's diff, as it always was.
async fn pr_diff(
    state: &AppState,
    workspace_id: &str,
    worktree: &Path,
    pr_base: &str,
    trunk: &str,
) -> crate::git::WorktreeDiff {
    let diff = if pr_base == trunk {
        state.diffs.compute(workspace_id, worktree).await
    } else {
        state
            .diffs
            .compute_against(workspace_id, worktree, pr_base)
            .await
    };
    diff.unwrap_or(crate::git::WorktreeDiff {
        workspace_id: workspace_id.to_string(),
        base: None,
        files: vec![],
    })
}

fn build_pr_body(
    why: &str,
    diff: &crate::git::WorktreeDiff,
    workspace: &WorkspaceRow,
    include_what: bool,
    include_tested: bool,
) -> String {
    let mut sections = vec![why.to_string()];

    if include_what {
        let mut lines = Vec::new();
        if diff.files.is_empty() {
            lines.push("No file changes in this workspace.".into());
        } else {
            for file in diff.files.iter().take(20) {
                lines.push(format!(
                    "- `{}` (+{} −{})",
                    file.path, file.added, file.deleted
                ));
            }
        }
        sections.push(format!("## What changed\n{}", lines.join("\n")));
    }

    if include_tested {
        let tested = workspace
            .summary
            .as_deref()
            .filter(|summary| summary.to_lowercase().contains("test"))
            .map(|summary| summary.to_string())
            .unwrap_or_else(|| {
                "Automated and manual checks from the agent run are included in the workspace thread."
                    .into()
            });
        sections.push(format!("## How tested\n{tested}"));
    }

    sections.join("\n\n")
}

fn diff_totals(diff: &crate::git::WorktreeDiff) -> (u32, u32, u32) {
    let mut added = 0u32;
    let mut deleted = 0u32;
    for file in &diff.files {
        added += file.added;
        deleted += file.deleted;
    }
    (added, deleted, diff.files.len() as u32)
}

fn append_opened_pr_step(
    store: &crate::store::Store,
    thread_id: &str,
    number: i64,
    branch: &str,
    base: &str,
) -> Result<()> {
    let event = AgentEvent::ToolCall {
        id: format!("pr-open-{number}"),
        title: format!("Opened PR #{number}"),
        name: None,
        kind: ToolKind::Other,
        status: ToolCallStatus::Completed,
        locations: vec![],
        detail: Some(format!("{branch} → {base}")),
    };
    let payload = serde_json::to_string(&event).map_err(|error| Error::Store(error.to_string()))?;
    store.append_event(thread_id, "tool", &payload)?;
    Ok(())
}

fn iso_timestamp_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

async fn refresh_git_stats(
    state: &AppState,
    workspace_id: &str,
    worktree: &Path,
    base: &str,
    branch: &str,
) {
    let behind = state
        .git
        .rev_list_count(worktree, &format!("HEAD..origin/{base}"))
        .await
        .unwrap_or(0);
    let ahead = state
        .git
        .rev_list_count(worktree, &format!("origin/{branch}..HEAD"))
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
