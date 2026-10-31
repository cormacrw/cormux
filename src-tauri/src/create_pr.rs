use std::path::Path;

use tauri::AppHandle;

use crate::engines::{AgentEvent, ToolCallStatus, ToolKind};
use crate::error::{Error, Result};
use crate::feedback::emit_toast;
use crate::git_workspace::{lead_thread_id, refresh_git_stats, resolve_record};
use crate::github::auth;
use crate::github::create::{CreatePullRequestInput, RestGithubClient};
use crate::github::types::{PrChecksState, PrRelationship, PrReviewState, PullRequestPayload};
use crate::ipc::types::{
    CreateWorkspacePullRequestInput, CreateWorkspacePullRequestResult, DraftPrWhyResult,
    StateChangeKind, ToastPart, ToastRaisedPayload, ToastTone,
};
use crate::pr_draft::{PR_PROMPT_KEY, draft_why, pr_prompt};
use crate::state::AppState;
use crate::store::types::{PrRow, WorkspaceRow};

/// A workspace that's still around, for the PR commands.
fn live_workspace(state: &AppState, workspace_id: &str) -> Result<WorkspaceRow> {
    state
        .store
        .workspace_by_id(workspace_id)?
        .filter(|row| row.archived_at.is_none())
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))
}

pub async fn draft_pr_why(state: &AppState, workspace_id: &str) -> Result<DraftPrWhyResult> {
    let workspace = live_workspace(state, workspace_id)?;
    if workspace.pr_number.is_some() {
        return Err(Error::Workspace("pull request already opened".into()));
    }

    let record = resolve_record(state, workspace_id, &workspace).await?;
    let worktree = Path::new(&record.worktree_path);
    let base = crate::stack::pr_base(state, worktree, &record.branch, &record.base).await;
    let diff = pr_diff(state, workspace_id, worktree, &base, &record.base).await;

    let lead_id = lead_thread_id(state, workspace_id)?;
    let goal = state.store.first_user_goal(&lead_id)?;
    let transcript = state.store.transcript_summary(&lead_id, 24)?;
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
    app: &AppHandle,
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
        .ok_or_else(|| Error::Github("GitHub is not signed in — run `gh auth login`".into()))?;

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

    state.store.set_workspace_pr(
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
        updated_at: iso_timestamp_now(),
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
                    value: format!("Opened draft PR #{} for ", created.number),
                },
                ToastPart::Code {
                    value: record.branch.clone(),
                },
            ],
            workspace_id: Some(input.workspace_id.clone()),
            thread_id: Some(lead_id.clone()),
        },
    );

    // One refetch covers the new PR row and the workspace's PR number.
    state.notify(app, StateChangeKind::PrSync);

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

fn append_opened_pr_step(
    store: &crate::store::Store,
    thread_id: &str,
    number: i64,
    branch: &str,
    base: &str,
) -> Result<()> {
    let event = AgentEvent::ToolCall {
        parent_id: None,
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

/// Now, in the `YYYY-MM-DDTHH:MM:SSZ` form GitHub's `updatedAt` takes, so the PR row made here
/// sorts and ages like the synced ones. (It used to be unix seconds, which the webview's
/// `Date.parse` couldn't read, so a fresh PR showed as updated at the epoch until the next sync.)
fn iso_timestamp_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    iso_timestamp(secs)
}

fn iso_timestamp(unix_secs: u64) -> String {
    let days = unix_secs / 86_400;
    let rem = unix_secs % 86_400;
    let (year, month, day) = civil_from_days(days as i64);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Days since 1970-01-01 to a proleptic Gregorian date (Howard Hinnant's `civil_from_days`).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_take_githubs_form() {
        assert_eq!(iso_timestamp(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_timestamp(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(iso_timestamp(1_727_657_551), "2024-09-30T00:52:31Z");
        assert_eq!(iso_timestamp(4_102_444_799), "2099-12-31T23:59:59Z");
    }
}
