use std::path::Path;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{Error, Result};
use crate::feedback::{Change, ToastPart, ToastRaisedPayload, ToastTone};
use crate::github::RestGithubClient;
use crate::github::auth;
use crate::github::create::CreatePullRequestInput;
use crate::github::types::{PrChecksState, PrRelationship, PrReviewState, PullRequestPayload};
use crate::pr_draft::{PR_PROMPT_KEY, draft_why, pr_prompt};
use crate::state::AppState;
use crate::store::types::{PrRow, WorkspaceRow};
use crate::timeline::TimelineStep;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DraftPrWhyResult {
    pub workspace_id: String,
    pub text: String,
    pub from_llm: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspacePullRequestInput {
    pub workspace_id: String,
    pub body: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspacePullRequestResult {
    pub workspace_id: String,
    pub number: i64,
    pub html_url: String,
    pub title: String,
}

/// A workspace that's still around, for the PR commands.
fn live_workspace(state: &AppState, workspace_id: &str) -> Result<WorkspaceRow> {
    state
        .store
        .workspace_by_id(workspace_id)?
        .filter(|row| row.archived_at.is_none())
        .ok_or_else(|| Error::unknown_workspace(workspace_id))
}

pub async fn draft_pr_why(state: &AppState, workspace_id: &str) -> Result<DraftPrWhyResult> {
    let workspace = live_workspace(state, workspace_id)?;
    if workspace.pr_number.is_some() {
        return Err(Error::Workspace("pull request already opened".into()));
    }

    let record = state.workspaces.get(workspace_id).await?;
    let worktree = Path::new(&record.worktree_path);
    let base = crate::stack::pr_base(state, worktree, &record.branch, &record.base).await;
    let diff = pr_diff(state, workspace_id, worktree, &base, &record.base).await;

    let thread = state
        .store
        .own_thread(workspace_id)?
        .ok_or_else(|| Error::Workspace(format!("no thread in workspace {workspace_id}")))?;
    let goal = state.store.first_user_goal(&thread.id)?;
    let transcript = state.store.transcript_summary(&thread.id, 24)?;
    let instructions = pr_prompt(state.store.get_setting(PR_PROMPT_KEY)?);

    let (text, from_llm) = draft_why(
        &state.llm,
        &instructions,
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
    state: &AppState,
    input: CreateWorkspacePullRequestInput,
) -> Result<CreateWorkspacePullRequestResult> {
    let body = input.body.trim();
    if body.is_empty() {
        return Err(Error::Workspace(
            "Add a description so reviewers know what this changes.".into(),
        ));
    }

    let workspace = live_workspace(state, &input.workspace_id)?;

    if workspace.pr_number.is_some() {
        return Err(Error::Workspace("pull request already opened".into()));
    }

    let record = state.workspaces.get(&input.workspace_id).await?;
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

    let token = auth::resolve_token(&state.shell_env).await?;

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

    crate::workspace_git::refresh_stats(state, &record).await;

    let origin = state
        .git
        .remote_origin_url(worktree)
        .await?
        .ok_or_else(|| Error::Git("no origin remote configured for this repo".into()))?;
    let slug = crate::github::parse_origin_url(&origin).ok_or_else(|| {
        Error::Git(format!(
            "could not parse origin URL {}",
            crate::github::redact_origin_url(&origin)
        ))
    })?;
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

    let client = RestGithubClient::new();
    let created = client
        .create_pull_request(
            &token,
            owner,
            repo,
            &CreatePullRequestInput {
                title: title.clone(),
                body: body.to_string(),
                head: record.branch.clone(),
                base: pr_base.clone(),
                draft: true,
            },
        )
        .await?;

    state.workspaces.set_pr(
        &input.workspace_id,
        Some(created.number),
        Some(&created.html_url),
    )?;

    let (additions, deletions, files) = diff_totals(&diff);
    let payload = PullRequestPayload {
        num: created.number,
        title: title.clone(),
        author: "you".into(),
        rel: PrRelationship::Author,
        head: record.branch.clone(),
        base: pr_base.clone(),
        // GitHub's form, so the row made here sorts and ages like the synced ones.
        updated_at: crate::time::iso_now(),
        checks: PrChecksState::Running,
        failing: None,
        review: PrReviewState::Draft,
        additions: additions as i64,
        deletions: deletions as i64,
        files: files as i64,
        is_draft: true,
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

    // The PR exists now, so nothing below may fail the command.
    crate::workspace_git::record_in_own_thread(
        state,
        &input.workspace_id,
        TimelineStep::PrOpened {
            number: created.number,
            branch: record.branch.clone(),
            base: pr_base.clone(),
        },
    );
    let own_thread = state
        .store
        .own_thread(&input.workspace_id)
        .ok()
        .flatten()
        .map(|thread| thread.id);

    state.out.toast(ToastRaisedPayload {
        tone: ToastTone::Ok,
        parts: vec![
            ToastPart::Text {
                value: format!("Opened draft PR #{} for ", created.number),
            },
            ToastPart::Code {
                value: record.branch.clone(),
            },
        ],
        workspace_id: Some(input.workspace_id.clone()),
        thread_id: own_thread,
    });

    // One refetch covers the new PR row and the workspace's PR number.
    state.out.changed(Change::SNAPSHOT);

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
        target: None,
        files: vec![],
        uncommitted: Default::default(),
    })
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
