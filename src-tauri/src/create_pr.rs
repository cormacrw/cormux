use std::path::Path;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{Error, Result};
use crate::feedback::{ToastPart, ToastRaisedPayload, ToastTone};
use crate::github::RepoSlug;
use crate::github::auth;
use crate::github::create::CreatePullRequestInput;
use crate::github::types::{OpenedPullRequest, PullRequestPayload};
use crate::pr_draft::{draft_why, pr_prompt};
use crate::state::AppState;
use crate::timeline::TimelineStep;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DraftPrWhyResult {
    pub workspace_id: String,
    pub text: String,
    pub from_llm: bool,
    /// The branch the PR will target: the base, or the branch below in a stack.
    pub base: String,
    /// What the PR will show, against `base`.
    pub files: u32,
    pub added: u32,
    pub deleted: u32,
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

async fn ensure_no_pr(state: &AppState, workspace_id: &str) -> Result<()> {
    if state.workspaces.pr_number(workspace_id).await?.is_some() {
        return Err(Error::Workspace("pull request already opened".into()));
    }
    Ok(())
}

pub async fn draft_pr_why(state: &AppState, workspace_id: &str) -> Result<DraftPrWhyResult> {
    ensure_no_pr(state, workspace_id).await?;
    let record = state.workspaces.get(workspace_id).await?;
    let worktree = Path::new(&record.worktree_path);
    let base = crate::stack::pr_base(state, worktree, &record.branch, &record.base).await?;
    let diff = pr_diff(state, workspace_id, worktree, &base).await?;

    let thread = state
        .store
        .own_thread(workspace_id)?
        .ok_or_else(|| Error::Workspace(format!("no thread in workspace {workspace_id}")))?;
    let goal = state.transcript.first_user_goal(&thread.id)?;
    let transcript = state.transcript.raw_tail(&thread.id, 24)?;
    let instructions = pr_prompt(&crate::settings::Settings::read(&state.store)?.pr_prompt);

    let (text, from_llm) = draft_why(
        &state.llm,
        &instructions,
        &record,
        &base,
        &goal,
        &transcript,
        &diff,
    )
    .await?;

    let (added, deleted, files) = diff_totals(&diff);
    Ok(DraftPrWhyResult {
        workspace_id: workspace_id.to_string(),
        text,
        from_llm,
        base,
        files,
        added,
        deleted,
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

    ensure_no_pr(state, &input.workspace_id).await?;
    let record = state.workspaces.get(&input.workspace_id).await?;
    let worktree = Path::new(&record.worktree_path);

    let porcelain = state.git.status_porcelain(worktree).await?;
    if !porcelain.trim().is_empty() {
        return Err(Error::Git(
            "commit or stash uncommitted changes before opening a pull request".into(),
        ));
    }

    // A stacked branch's PR targets the branch below it, which GitHub needs on the remote.
    let pr_base = crate::stack::pr_base(state, worktree, &record.branch, &record.base).await?;
    if pr_base != record.base && !state.git.remote_branch_exists(worktree, &pr_base).await? {
        return Err(Error::Git(format!(
            "{} is stacked on {pr_base}, which isn't on GitHub yet. Push the stack from the Stack tab, then try again.",
            record.branch
        )));
    }

    let token = auth::resolve_token(&state.github.gh).await?;

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

    let slug = RepoSlug::of_origin(&state.git, worktree).await?;

    let diff = pr_diff(state, &input.workspace_id, worktree, &pr_base).await?;

    let title = input
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(record.name.as_str())
        .to_string();

    let created = state
        .github
        .rest
        .create_pull_request(
            &token,
            &slug.owner,
            &slug.name,
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

    let (_, _, files) = diff_totals(&diff);
    state
        .store
        .upsert_pr(&PullRequestPayload::opened_by_viewer(OpenedPullRequest {
            number: created.number,
            title: title.clone(),
            head: record.branch.clone(),
            base: pr_base.clone(),
            files: files as i64,
            html_url: created.html_url.clone(),
            repo_full_name: slug.full_name(),
            repo_id: record.repo_id.clone(),
        }))?;

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
        ..Default::default()
    });

    // One refetch covers the new PR row and the workspace's PR number.
    state.out.changed();

    Ok(CreateWorkspacePullRequestResult {
        workspace_id: input.workspace_id,
        number: created.number,
        html_url: created.html_url,
        title,
    })
}

/// The changes the PR will show: the branch's commits against the branch it targets, whatever
/// the Changes panel is showing.
async fn pr_diff(
    state: &AppState,
    workspace_id: &str,
    worktree: &Path,
    pr_base: &str,
) -> Result<crate::git::WorktreeDiff> {
    state
        .diffs
        .compute_against(workspace_id, worktree, pr_base)
        .await
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

pub mod commands {
    use tauri::State;

    use crate::error::Result;
    use crate::state::AppState;

    use super::{
        CreateWorkspacePullRequestInput, CreateWorkspacePullRequestResult, DraftPrWhyResult,
    };

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
        super::draft_pr_why(&state, &workspace_id).await
    }

    #[tauri::command]
    #[specta::specta]
    pub async fn create_workspace_pull_request(
        input: CreateWorkspacePullRequestInput,
        state: State<'_, AppState>,
    ) -> Result<CreateWorkspacePullRequestResult> {
        super::create_workspace_pull_request(&state, input).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(dir: &Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(output.status.success(), "git {args:?}: {output:?}");
    }

    #[tokio::test]
    async fn the_pr_diff_is_the_branch_against_its_base_whatever_changes_shows() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        git(repo, &["init", "-q", "-b", "main"]);
        git(repo, &["config", "user.email", "t@example.com"]);
        git(repo, &["config", "user.name", "T"]);
        std::fs::write(repo.join("a.txt"), "a\n").unwrap();
        git(repo, &["add", "."]);
        git(repo, &["commit", "-qm", "base"]);
        git(repo, &["switch", "-qc", "feat"]);
        std::fs::write(repo.join("b.txt"), "one\ntwo\n").unwrap();
        git(repo, &["add", "."]);
        git(repo, &["commit", "-qm", "work"]);
        std::fs::write(repo.join("a.txt"), "a\nuncommitted\n").unwrap();
        let (state, _recording) = AppState::recorded(crate::paths::DataDir::new(repo.into()));
        state.diffs.set_diff_target("w1", None);

        let diff = pr_diff(&state, "w1", repo, "main").await.unwrap();

        let files: Vec<&str> = diff.files.iter().map(|file| file.path.as_str()).collect();
        assert_eq!(files, ["b.txt"]);
        assert_eq!(diff_totals(&diff), (2, 0, 1));
    }
}
