//! Stacked pull requests through the `gh stack` CLI extension (github/gh-stack).
//!
//! gh-stack owns the stack: it keeps it in `$(git rev-parse --git-dir)/gh-stack`, which is
//! per worktree, so each workspace has its own stack. Cormux reads it with
//! `gh stack view --json`, adds line counts from git, and drives `init`, `add`, `push`
//! and `sync` with their non-interactive flags. Pull requests are opened by hand with
//! Create PR, never by gh-stack. Exit codes are gh-stack's documented ones.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::{LazyLock, Mutex};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;

use crate::error::{Error, Result};
use crate::feedback::emit_toast;
use crate::git::ShortStat;
use crate::git_workspace::{adopt_checked_out_branch, resolve_record};
use crate::ipc::types::{ToastPart, ToastRaisedPayload, ToastTone};
use crate::state::AppState;
use crate::workspace::WorkspaceRecord;

const EXIT_NOT_IN_STACK: i32 = 2;
/// `gh stack checkout`: no stack, local or on GitHub, has the branch.
const EXIT_STACK_NOT_FOUND: i32 = 2;
const EXIT_REBASE_CONFLICT: i32 = 3;
const EXIT_GITHUB_API: i32 = 4;
const EXIT_LOCKED: i32 = 8;
const EXIT_UNAVAILABLE: i32 = 9;

pub const INSTALL_HINT: &str = "gh extension install github/gh-stack";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum StackStatus {
    /// The checked-out branch is in a gh-stack stack.
    Stacked,
    /// gh-stack works here, but the checked-out branch isn't in a stack.
    NotStacked,
    /// `gh` or the gh-stack extension is missing, or stacks are off for this repo.
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StackPullRequest {
    pub number: u32,
    pub url: Option<String>,
    /// `OPEN`, `MERGED` or `QUEUED`, as gh-stack reports it.
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StackBranch {
    pub name: String,
    /// The branch this one's pull request targets: the nearest unmerged branch below, or the trunk.
    pub parent: String,
    pub files: u32,
    pub additions: u32,
    pub deletions: u32,
    pub commits: u32,
    pub current: bool,
    pub merged: bool,
    pub queued: bool,
    pub needs_rebase: bool,
    pub pr: Option<StackPullRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceStack {
    pub workspace_id: String,
    pub status: StackStatus,
    /// Why the stack is unavailable, for the panel to show.
    pub message: Option<String>,
    pub trunk: String,
    pub current_branch: String,
    /// Bottom of the stack (closest to the trunk) first.
    pub branches: Vec<StackBranch>,
}

/// `gh stack view --json`, per gh-stack v0.1's `viewJSONOutput`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ViewJson {
    trunk: String,
    current_branch: String,
    #[serde(default)]
    branches: Vec<ViewBranch>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ViewBranch {
    name: String,
    #[serde(default)]
    is_merged: bool,
    #[serde(default)]
    is_queued: bool,
    #[serde(default)]
    needs_rebase: bool,
    pr: Option<ViewPr>,
}

#[derive(Debug, Deserialize)]
struct ViewPr {
    number: u32,
    url: Option<String>,
    #[serde(default)]
    state: String,
}

enum View {
    Stacked(ViewJson),
    NotStacked,
    Unavailable(String),
}

async fn gh_stack(
    state: &AppState,
    worktree: &Path,
    args: &[&str],
    token: Option<&str>,
) -> Result<Output> {
    let mut full = vec!["stack"];
    full.extend_from_slice(args);
    // Never let gh or gh-stack wait on a prompt; there's no terminal to answer it.
    let mut env = vec![("GH_PROMPT_DISABLED", "1"), ("NO_COLOR", "1")];
    if let Some(token) = token {
        env.push(("GH_TOKEN", token));
    }
    state
        .git
        .run_tool(worktree, "gh", &full, &env)
        .await
        .map_err(|_| {
            Error::Git(format!(
                "GitHub CLI not found. Install gh, then {INSTALL_HINT}."
            ))
        })
}

fn stderr_text(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let text = if stderr.trim().is_empty() {
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        stderr.to_string()
    };
    text.lines()
        .map(|line| line.trim_start_matches(['✗', '!', ' ']).trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn extension_missing(output: &Output) -> bool {
    let stderr = String::from_utf8_lossy(&output.stderr);
    stderr.contains("unknown command \"stack\"")
}

/// Turn a failed gh-stack run into an error the UI can show as-is.
fn failure(output: &Output, action: &str) -> Error {
    if extension_missing(output) {
        return Error::Git(format!("gh-stack isn't installed. Run: {INSTALL_HINT}"));
    }
    let detail = stderr_text(output);
    let message = match output.status.code() {
        Some(EXIT_REBASE_CONFLICT) => format!(
            "{action} stopped on a rebase conflict and put the stack back. Run `gh stack rebase` in the worktree to resolve it."
        ),
        Some(EXIT_GITHUB_API) => {
            format!(
                "{action} failed talking to GitHub. Check `gh auth status`, then retry. {detail}"
            )
        }
        Some(EXIT_LOCKED) => {
            format!("{action} is waiting on another gh stack command. Try again in a few seconds.")
        }
        Some(EXIT_UNAVAILABLE) => "Stacked pull requests aren't enabled on this repository.".into(),
        _ if detail.is_empty() => format!("{action} failed"),
        _ => format!("{action} failed: {detail}"),
    };
    Error::Git(message)
}

async fn view(state: &AppState, worktree: &Path) -> Result<View> {
    let output = match gh_stack(state, worktree, &["view", "--json"], None).await {
        Ok(output) => output,
        Err(Error::Git(message)) => return Ok(View::Unavailable(message)),
        Err(error) => return Err(error),
    };
    if output.status.success() {
        let parsed: ViewJson = serde_json::from_slice(&output.stdout)
            .map_err(|error| Error::Git(format!("could not read gh stack view --json: {error}")))?;
        return Ok(View::Stacked(parsed));
    }
    if extension_missing(&output) {
        return Ok(View::Unavailable(format!(
            "The gh-stack extension isn't installed. Run: {INSTALL_HINT}"
        )));
    }
    match output.status.code() {
        Some(EXIT_NOT_IN_STACK) => Ok(View::NotStacked),
        Some(EXIT_UNAVAILABLE) => Ok(View::Unavailable(
            "Stacked pull requests aren't enabled on this repository.".into(),
        )),
        _ => Err(failure(&output, "Reading the stack")),
    }
}

async fn workspace_record(state: &AppState, workspace_id: &str) -> Result<WorkspaceRecord> {
    let row = state
        .store
        .workspace_by_id(workspace_id)?
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
    resolve_record(state, workspace_id, &row).await
}

/// Each unmerged branch's pull request targets the nearest unmerged branch below it.
pub fn parents(names_and_merged: &[(String, bool)], trunk: &str) -> Vec<String> {
    let mut below = trunk.to_string();
    names_and_merged
        .iter()
        .map(|(name, merged)| {
            let parent = below.clone();
            if !merged {
                below = name.clone();
            }
            parent
        })
        .collect()
}

pub async fn workspace_stack(state: &AppState, workspace_id: &str) -> Result<WorkspaceStack> {
    let record = workspace_record(state, workspace_id).await?;
    let worktree = PathBuf::from(&record.worktree_path);
    let empty = |status, message| WorkspaceStack {
        workspace_id: workspace_id.to_string(),
        status,
        message,
        trunk: record.base.clone(),
        current_branch: record.branch.clone(),
        branches: Vec::new(),
    };

    let json = match view(state, &worktree).await? {
        View::Stacked(json) => json,
        View::NotStacked => {
            if !checkout_remote_stack(state, &worktree, &record.branch, &record.base).await {
                return Ok(empty(StackStatus::NotStacked, None));
            }
            match view(state, &worktree).await? {
                View::Stacked(json) => json,
                _ => return Ok(empty(StackStatus::NotStacked, None)),
            }
        }
        View::Unavailable(message) => return Ok(empty(StackStatus::Unavailable, Some(message))),
    };

    let trunk_ref = trunk_ref(state, &worktree, &json.trunk).await;
    let order: Vec<(String, bool)> = json
        .branches
        .iter()
        .map(|branch| (branch.name.clone(), branch.is_merged))
        .collect();
    let parents = parents(&order, &json.trunk);

    let mut branches = Vec::with_capacity(json.branches.len());
    for (branch, parent) in json.branches.into_iter().zip(parents) {
        let (stat, commits) = if branch.is_merged {
            (ShortStat::default(), 0)
        } else {
            let base = if parent == json.trunk {
                trunk_ref.clone()
            } else {
                parent.clone()
            };
            let stat = state
                .git
                .shortstat(&worktree, &format!("{base}...{}", branch.name))
                .await
                .unwrap_or_default();
            let commits = state
                .git
                .rev_list_count(&worktree, &format!("{base}..{}", branch.name))
                .await
                .unwrap_or(0);
            (stat, commits)
        };
        branches.push(StackBranch {
            current: branch.name == json.current_branch,
            name: branch.name,
            parent,
            files: stat.files,
            additions: stat.added,
            deletions: stat.deleted,
            commits,
            merged: branch.is_merged,
            queued: branch.is_queued,
            needs_rebase: branch.needs_rebase,
            pr: branch.pr.map(|pr| StackPullRequest {
                number: pr.number,
                url: pr.url,
                state: pr.state,
            }),
        });
    }

    Ok(WorkspaceStack {
        workspace_id: workspace_id.to_string(),
        status: StackStatus::Stacked,
        message: None,
        trunk: json.trunk,
        current_branch: json.current_branch,
        branches,
    })
}

/// The branch each worktree last looked for on GitHub, so reopening the Stack tab doesn't
/// ask again until a different branch is checked out.
static REMOTE_STACK_CHECKED: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(Default::default);

/// A branch checked out from GitHub has no gh-stack metadata in this worktree, so
/// `gh stack view` calls it unstacked. `gh stack checkout <branch>` looks for it in the
/// stacks on GitHub too and, when one has it, sets that stack up here. Returns whether it did.
async fn checkout_remote_stack(
    state: &AppState,
    worktree: &Path,
    branch: &str,
    trunk: &str,
) -> bool {
    if branch.is_empty() || branch == trunk {
        return false;
    }
    let key = worktree.to_string_lossy().to_string();
    {
        let mut checked = REMOTE_STACK_CHECKED
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if checked.get(&key).is_some_and(|last| last == branch) {
            return false;
        }
        checked.insert(key.clone(), branch.to_string());
    }
    let forget = || {
        // Look again next time: this failure says nothing about whether a stack exists.
        REMOTE_STACK_CHECKED
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&key);
    };

    let token = fallback_token(state).await;
    let output = match gh_stack(state, worktree, &["checkout", branch], token.as_deref()).await {
        Ok(output) => output,
        Err(_) => {
            forget();
            return false;
        }
    };
    match output.status.code() {
        Some(0) => {}
        Some(EXIT_STACK_NOT_FOUND) => return false,
        _ => {
            log::warn!("gh stack checkout {branch}: {}", stderr_text(&output));
            forget();
            return false;
        }
    }
    // gh-stack checks the branch out as part of setting up the stack; keep the workspace
    // on the branch it asked for.
    if state.git.current_branch(worktree).await.ok().as_deref() != Some(branch)
        && let Err(error) = state.git.switch(worktree, branch).await
    {
        log::warn!("switching back to {branch} after gh stack checkout: {error}");
    }
    true
}

/// Add `branch` on top of the stack. When the checked-out branch isn't stacked yet,
/// start a stack first: on the trunk the new branch becomes its bottom, otherwise the
/// checked-out branch is adopted as the bottom and the new branch goes on top of it.
pub async fn add_branch(
    app: &AppHandle,
    state: &AppState,
    workspace_id: &str,
    branch: &str,
) -> Result<()> {
    let name = branch.trim();
    if name.is_empty() {
        return Err(Error::Git("branch name is required".into()));
    }
    let record = workspace_record(state, workspace_id).await?;
    state.workspace.remember(record.clone()).await;
    state.workspace.can_switch_branch(workspace_id).await?;
    let worktree = PathBuf::from(&record.worktree_path);

    match view(state, &worktree).await? {
        View::Unavailable(message) => return Err(Error::Git(message)),
        View::Stacked(json) => {
            if json.branches.last().map(|top| &top.name) != Some(&json.current_branch) {
                return Err(Error::Git(
                    "New branches go on top of the stack. Check out the top branch first.".into(),
                ));
            }
            run(state, &worktree, &["add", name], "Adding the branch").await?;
        }
        View::NotStacked if record.branch == record.base => {
            run(
                state,
                &worktree,
                &["init", "--base", &record.base, name],
                "Starting the stack",
            )
            .await?;
        }
        View::NotStacked => {
            run(
                state,
                &worktree,
                &["init", "--base", &record.base, &record.branch],
                "Starting the stack",
            )
            .await?;
            run(state, &worktree, &["add", name], "Adding the branch").await?;
        }
    }

    adopt_checked_out_branch(
        app,
        state,
        workspace_id,
        &format!("Added {name} to the stack on top of {}", record.branch),
    )
    .await?;
    Ok(())
}

/// Push every unmerged branch in the stack (`--force-with-lease` per branch). Pull
/// requests stay manual: this never opens or edits one.
pub async fn push(app: &AppHandle, state: &AppState, workspace_id: &str) -> Result<()> {
    let record = workspace_record(state, workspace_id).await?;
    let worktree = PathBuf::from(&record.worktree_path);
    let output = gh_stack(state, &worktree, &["push", "--remote", "origin"], None).await?;
    if !output.status.success() {
        return Err(failure(&output, "Push"));
    }
    adopt_checked_out_branch(app, state, workspace_id, "Pushed the stack").await?;
    toast(app, workspace_id, "Pushed the stack");
    Ok(())
}

/// Fetch, rebase onto the trunk, push, and refresh pull request state.
pub async fn sync(app: &AppHandle, state: &AppState, workspace_id: &str) -> Result<()> {
    let record = workspace_record(state, workspace_id).await?;
    state.workspace.remember(record.clone()).await;
    state.workspace.can_switch_branch(workspace_id).await?;
    let worktree = PathBuf::from(&record.worktree_path);
    let token = fallback_token(state).await;
    let output = gh_stack(
        state,
        &worktree,
        &["sync", "--remote", "origin"],
        token.as_deref(),
    )
    .await?;
    let result = if !output.status.success() {
        Err(failure(&output, "Sync"))
    } else if sync_aborted(&output) {
        // Without a terminal, a divergence makes no changes and still exits 0.
        Err(Error::Git(
            "Sync stopped: the stack on GitHub and this worktree have diverged. Run `gh stack sync` in a terminal to choose which to keep.".into(),
        ))
    } else {
        Ok(())
    };
    // A sync can rebase the checked-out branch even when it fails partway.
    adopt_checked_out_branch(app, state, workspace_id, "Synced the stack").await?;
    result?;
    toast(app, workspace_id, "Synced the stack with GitHub");
    Ok(())
}

/// Cormux's own token, but only when `gh` isn't signed in; a working `gh` login wins.
async fn fallback_token(state: &AppState) -> Option<String> {
    let signed_in = state
        .shell_env
        .read()
        .await
        .run("gh", &["auth", "token"], None)
        .await
        .is_ok_and(|token| !token.trim().is_empty());
    if signed_in {
        None
    } else {
        crate::github::auth::read_stored_token()
    }
}

fn sync_aborted(output: &Output) -> bool {
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    text.contains("Sync aborted")
}

async fn run(state: &AppState, worktree: &Path, args: &[&str], action: &str) -> Result<()> {
    let output = gh_stack(state, worktree, args, None).await?;
    if output.status.success() {
        Ok(())
    } else {
        Err(failure(&output, action))
    }
}

/// The branch a pull request for `branch` should target: the nearest unmerged branch
/// below it when it's stacked, otherwise `trunk`.
pub async fn pr_base(state: &AppState, worktree: &Path, branch: &str, trunk: &str) -> String {
    let Ok(View::Stacked(json)) = view(state, worktree).await else {
        return trunk.to_string();
    };
    let order: Vec<(String, bool)> = json
        .branches
        .iter()
        .map(|row| (row.name.clone(), row.is_merged))
        .collect();
    let parents = parents(&order, &json.trunk);
    order
        .iter()
        .position(|(name, merged)| name == branch && !merged)
        .map(|index| parents[index].clone())
        .unwrap_or_else(|| trunk.to_string())
}

fn toast(app: &AppHandle, workspace_id: &str, text: &str) {
    emit_toast(
        app,
        ToastRaisedPayload {
            tone: ToastTone::Ok,
            parts: vec![ToastPart::Text { value: text.into() }],
            workspace_id: Some(workspace_id.to_string()),
        },
    );
}

/// Compare against the remote trunk when it exists, so the bottom branch doesn't
/// count commits that only a stale local trunk is missing.
async fn trunk_ref(state: &AppState, worktree: &Path, trunk: &str) -> String {
    let remote = format!("origin/{trunk}");
    if state
        .git
        .ref_exists(worktree, &remote)
        .await
        .unwrap_or(false)
    {
        remote
    } else {
        trunk.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(rows: &[(&str, bool)]) -> Vec<(String, bool)> {
        rows.iter()
            .map(|(name, merged)| (name.to_string(), *merged))
            .collect()
    }

    #[test]
    fn each_branch_targets_the_nearest_unmerged_branch_below() {
        let rows = order(&[("a", false), ("b", false), ("c", false)]);
        assert_eq!(parents(&rows, "main"), ["main", "a", "b"]);
    }

    #[test]
    fn merged_branches_drop_out_of_the_chain() {
        let rows = order(&[("a", true), ("b", false), ("c", true), ("d", false)]);
        assert_eq!(parents(&rows, "main"), ["main", "main", "b", "b"]);
    }

    fn git_cmd(cwd: &Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(cwd)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }

    /// Drives the real gh-stack CLI in a worktree: `gh extension install github/gh-stack`.
    #[tokio::test]
    #[ignore = "needs gh with the gh-stack extension"]
    async fn gh_stack_in_a_worktree() {
        use crate::shell_env::ShellEnv;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let mut env = ShellEnv::new();
        env.load_or_inherit().await.unwrap();
        let git = crate::git::Git::new(Arc::new(RwLock::new(env)));
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir(&repo).unwrap();
        git_cmd(&repo, &["init", "-q", "-b", "main"]);
        git_cmd(
            &repo,
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                "init",
            ],
        );
        let worktree = dir.path().join("wt");
        git_cmd(
            &repo,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "feat/a",
                worktree.to_str().unwrap(),
            ],
        );

        let quiet = [("GH_PROMPT_DISABLED", "1"), ("NO_COLOR", "1")];
        let view = git
            .run_tool(&worktree, "gh", &["stack", "view", "--json"], &quiet)
            .await
            .unwrap();
        assert_eq!(view.status.code(), Some(EXIT_NOT_IN_STACK));

        let init = git
            .run_tool(
                &worktree,
                "gh",
                &["stack", "init", "--base", "main", "feat/a"],
                &quiet,
            )
            .await
            .unwrap();
        assert!(
            init.status.success(),
            "{}",
            String::from_utf8_lossy(&init.stderr)
        );
        std::fs::write(worktree.join("a.txt"), "one\ntwo\n").unwrap();
        git_cmd(&worktree, &["add", "a.txt"]);
        git_cmd(
            &worktree,
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-q",
                "-m",
                "a",
            ],
        );
        let add = git
            .run_tool(&worktree, "gh", &["stack", "add", "feat/b"], &quiet)
            .await
            .unwrap();
        assert!(
            add.status.success(),
            "{}",
            String::from_utf8_lossy(&add.stderr)
        );

        let view = git
            .run_tool(&worktree, "gh", &["stack", "view", "--json"], &quiet)
            .await
            .unwrap();
        assert!(view.status.success());
        let json: ViewJson = serde_json::from_slice(&view.stdout).unwrap();
        assert_eq!(json.trunk, "main");
        assert_eq!(json.current_branch, "feat/b");
        let names: Vec<_> = json
            .branches
            .iter()
            .map(|branch| branch.name.as_str())
            .collect();
        assert_eq!(names, ["feat/a", "feat/b"]);
        assert_eq!(git.current_branch(&worktree).await.unwrap(), "feat/b");
        let stat = git.shortstat(&worktree, "main...feat/a").await.unwrap();
        assert_eq!((stat.files, stat.added), (1, 2));

        // Only the top of the stack takes new branches.
        git_cmd(&worktree, &["switch", "-q", "feat/a"]);
        let add = git
            .run_tool(&worktree, "gh", &["stack", "add", "feat/c"], &quiet)
            .await
            .unwrap();
        assert!(!add.status.success());
    }

    #[test]
    fn parses_gh_stack_view_json() {
        let json = r#"{
          "trunk": "main",
          "currentBranch": "b",
          "branches": [
            {"name": "a", "head": "1", "base": "0", "isCurrent": false, "isMerged": true,
             "isQueued": false, "needsRebase": false,
             "pr": {"number": 7, "url": "https://github.com/o/r/pull/7", "state": "MERGED"}},
            {"name": "b", "base": "1", "isCurrent": true, "isMerged": false,
             "isQueued": false, "needsRebase": true}
          ]
        }"#;
        let view: ViewJson = serde_json::from_str(json).unwrap();
        assert_eq!(view.current_branch, "b");
        assert_eq!(view.branches[0].pr.as_ref().unwrap().number, 7);
        assert!(view.branches[0].is_merged);
        assert!(view.branches[1].needs_rebase);
        assert!(view.branches[1].pr.is_none());
    }
}
