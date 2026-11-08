//! Stacked pull requests through the `gh stack` CLI extension (github/gh-stack).
//!
//! gh-stack owns the stack: it keeps it in `$(git rev-parse --git-dir)/gh-stack`, which is
//! per worktree, so each workspace has its own stack. Cormux reads it with
//! `gh stack view --json`, adds line counts from git, and drives `init`, `add`, `push`
//! and `sync` with their non-interactive flags. Pull requests are opened by hand with
//! Create PR, never by gh-stack. Exit codes are gh-stack's documented ones.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Output as CommandOutput;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{Error, Result};
use crate::feedback::{ToastPart, ToastRaisedPayload, ToastTone};
use crate::git::ShortStat;
use crate::github::gh::{GhFailure, NOT_INSTALLED};
use crate::state::AppState;
use crate::timeline::BranchChange;
use crate::workspace_git::adopt_checked_out_branch;
use crate::workspaces::WorkspaceRecord;

/// How long GitHub's "no stack has this branch" stands before asking again.
const REMOTE_STACK_TTL: Duration = Duration::from_secs(120);

/// `gh stack view`: the branch isn't in a stack. `gh stack checkout`: no stack, local or on
/// GitHub, has it.
const EXIT_NO_STACK: i32 = 2;
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
    pub state: StackPrState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum StackPrState {
    Open,
    Merged,
    Queued,
}

impl StackPrState {
    /// From gh-stack's word (`OPEN`, `MERGED`, `QUEUED`); anything else reads as open.
    fn from_gh(state: &str) -> Self {
        match state {
            "MERGED" => Self::Merged,
            "QUEUED" => Self::Queued,
            _ => Self::Open,
        }
    }
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
    /// What to run to make stacks available, when installing something would.
    pub install_command: Option<String>,
    pub trunk: String,
    pub current_branch: String,
    /// The checked-out branch is missing commits from the branch it's based on: the
    /// one below it in the stack, or the trunk when it isn't stacked.
    pub current_needs_rebase: bool,
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

impl ViewJson {
    /// Each branch's parent, bottom first.
    fn parents(&self) -> Vec<String> {
        let order: Vec<(String, bool)> = self
            .branches
            .iter()
            .map(|branch| (branch.name.clone(), branch.is_merged))
            .collect();
        parents(&order, &self.trunk)
    }
}

enum View {
    Stacked(ViewJson),
    NotStacked,
    Unavailable(Unavailable),
}

/// Why stacks can't be used in a worktree, and the command that would fix it, if one would.
#[derive(Debug, Clone, PartialEq)]
struct Unavailable {
    reason: String,
    install: Option<&'static str>,
}

impl Unavailable {
    fn because(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            install: None,
        }
    }

    fn extension_missing() -> Self {
        Self {
            reason: "The gh-stack extension isn't installed.".into(),
            install: Some(INSTALL_HINT),
        }
    }

    /// As an error's text, for actions that can't go ahead.
    fn text(&self) -> String {
        match self.install {
            Some(command) => format!("{} Run: {command}", self.reason),
            None => self.reason.clone(),
        }
    }
}

async fn gh_stack(state: &AppState, worktree: &Path, args: &[&str]) -> Result<CommandOutput> {
    gh_stack_with_env(state, worktree, args, &[]).await
}

/// `gh stack args…`, queued with the worktree's git calls since it rewrites branches.
async fn gh_stack_with_env(
    state: &AppState,
    worktree: &Path,
    args: &[&str],
    extra_env: &[(&str, &str)],
) -> Result<CommandOutput> {
    let mut full = vec!["stack"];
    full.extend_from_slice(args);
    state
        .git
        .queue()
        .run(worktree, || {
            state.github.gh.output(Some(worktree), &full, extra_env)
        })
        .await
        .map_err(gh_failure)
}

fn gh_failure(failure: GhFailure) -> Error {
    match failure {
        GhFailure::NotInstalled => Error::Github(not_installed()),
        other => other.into(),
    }
}

fn not_installed() -> String {
    format!("{NOT_INSTALLED}, then {INSTALL_HINT}.")
}

fn stderr_text(output: &CommandOutput) -> String {
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

fn extension_missing(output: &CommandOutput) -> bool {
    stderr_says_extension_missing(&String::from_utf8_lossy(&output.stderr))
}

/// Older gh says `unknown command "stack"`; gh 2.103+ suggests installing the official extension.
fn stderr_says_extension_missing(stderr: &str) -> bool {
    stderr.contains("unknown command \"stack\"") || stderr.contains(INSTALL_HINT)
}

/// Turn a failed gh-stack run into an error the UI can show as-is.
fn failure(output: &CommandOutput, action: &str) -> Error {
    Error::Git(failure_message(output, action))
}

fn failure_message(output: &CommandOutput, action: &str) -> String {
    if extension_missing(output) {
        return format!("gh-stack isn't installed. Run: {INSTALL_HINT}");
    }
    let detail = stderr_text(output);
    match output.status.code() {
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
    }
}

async fn view(state: &AppState, worktree: &Path) -> Result<View> {
    // Reads local state only, so it needn't queue behind the worktree's git calls.
    let args = ["stack", "view", "--json"];
    let output = match state.github.gh.output(Some(worktree), &args, &[]).await {
        Ok(output) => output,
        Err(GhFailure::NotInstalled) => {
            return Ok(View::Unavailable(Unavailable::because(not_installed())));
        }
        Err(failure) => return Err(failure.into()),
    };
    if output.status.success() {
        let parsed: ViewJson = serde_json::from_slice(&output.stdout)
            .map_err(|error| Error::Git(format!("could not read gh stack view --json: {error}")))?;
        return Ok(View::Stacked(parsed));
    }
    if extension_missing(&output) {
        return Ok(View::Unavailable(Unavailable::extension_missing()));
    }
    match output.status.code() {
        Some(EXIT_NO_STACK) => Ok(View::NotStacked),
        Some(EXIT_UNAVAILABLE) => Ok(View::Unavailable(Unavailable::because(
            "Stacked pull requests aren't enabled on this repository.",
        ))),
        _ => Err(failure(&output, "Reading the stack")),
    }
}

async fn workspace_record(state: &AppState, workspace_id: &str) -> Result<WorkspaceRecord> {
    state.workspaces.get(workspace_id).await
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

/// The workspace's stack. When its branch reads as unstacked, a stack made in another worktree
/// or on GitHub is set up here first (`gh stack init`/`checkout`), which writes to the
/// worktree: the workspace record follows whatever branch that leaves checked out.
pub async fn workspace_stack(state: &AppState, workspace_id: &str) -> Result<WorkspaceStack> {
    let record = workspace_record(state, workspace_id).await?;
    let worktree = PathBuf::from(&record.worktree_path);
    let solo_needs_rebase = record.branch != record.base && {
        let trunk = state.git.freshest_ref(&worktree, &record.base).await;
        !state.git.is_ancestor(&worktree, &trunk, "HEAD").await
    };
    let empty = |status, message| WorkspaceStack {
        workspace_id: workspace_id.to_string(),
        status,
        message,
        install_command: None,
        trunk: record.base.clone(),
        current_branch: record.branch.clone(),
        current_needs_rebase: solo_needs_rebase,
        branches: Vec::new(),
    };

    let json = match view(state, &worktree).await? {
        View::Stacked(json) => json,
        View::NotStacked => {
            if !adopt_local_stack(state, &worktree, &record.branch).await {
                if state.stacks.recently_unstacked(&worktree, &record.branch) {
                    return Ok(empty(StackStatus::NotStacked, None));
                }
                match checkout_remote_stack(state, &worktree, &record.branch, &record.base).await {
                    RemoteStack::CheckedOut => {}
                    RemoteStack::None => {
                        state.stacks.remember_unstacked(&worktree, &record.branch);
                        return Ok(empty(StackStatus::NotStacked, None));
                    }
                    RemoteStack::Failed(message) => {
                        return Ok(empty(StackStatus::NotStacked, Some(message)));
                    }
                }
            }
            // Setting the stack up switches branches and back; keep the record on whatever
            // that left checked out.
            adopt_checked_out_branch(
                state,
                workspace_id,
                BranchChange::CheckedOutOutside {
                    previous: record.branch.clone(),
                },
            )
            .await?;
            match view(state, &worktree).await? {
                View::Stacked(json) => json,
                _ => return Ok(empty(StackStatus::NotStacked, None)),
            }
        }
        View::Unavailable(unavailable) => {
            return Ok(WorkspaceStack {
                install_command: unavailable.install.map(Into::into),
                ..empty(StackStatus::Unavailable, Some(unavailable.reason))
            });
        }
    };

    let trunk_ref = state.git.freshest_ref(&worktree, &json.trunk).await;
    let parents = json.parents();

    let mut branches = Vec::with_capacity(json.branches.len());
    for (branch, parent) in json.branches.into_iter().zip(parents) {
        let (stat, commits, behind) = if branch.is_merged {
            (ShortStat::default(), 0, false)
        } else {
            let base = if parent == json.trunk {
                trunk_ref.clone()
            } else {
                parent.clone()
            };
            // A branch git can't count (deleted under the stack) still lists, without numbers.
            let stat = state
                .git
                .shortstat(&worktree, &format!("{base}...{}", branch.name))
                .await
                .unwrap_or_else(|error| {
                    log::warn!("stack line counts for {}: {error}", branch.name);
                    ShortStat::default()
                });
            let commits = state
                .git
                .rev_list_count(&worktree, &format!("{base}..{}", branch.name))
                .await
                .unwrap_or_else(|error| {
                    log::warn!("stack commit count for {}: {error}", branch.name);
                    0
                });
            let behind = !state.git.is_ancestor(&worktree, &base, &branch.name).await;
            (stat, commits, behind)
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
            needs_rebase: branch.needs_rebase || behind,
            pr: branch.pr.map(|pr| StackPullRequest {
                number: pr.number,
                url: pr.url,
                state: StackPrState::from_gh(&pr.state),
            }),
        });
    }

    let current_needs_rebase = branches
        .iter()
        .any(|branch| branch.current && branch.needs_rebase);
    Ok(WorkspaceStack {
        workspace_id: workspace_id.to_string(),
        status: StackStatus::Stacked,
        message: None,
        install_command: None,
        current_needs_rebase,
        trunk: json.trunk,
        current_branch: json.current_branch,
        branches,
    })
}

/// gh-stack's state file, `$(git rev-parse --git-dir)/gh-stack`; only the fields read here.
#[derive(Debug, Deserialize)]
struct StackFile {
    #[serde(default)]
    stacks: Vec<StackFileStack>,
}

#[derive(Debug, Deserialize)]
struct StackFileStack {
    trunk: StackFileRef,
    #[serde(default)]
    branches: Vec<StackFileRef>,
}

#[derive(Debug, Deserialize)]
struct StackFileRef {
    branch: String,
}

/// The trunk and branches (bottom first) of the stack in `file` that has `branch`.
fn stack_with_branch(file: &str, branch: &str) -> Option<(String, Vec<String>)> {
    let parsed: StackFile = serde_json::from_str(file).ok()?;
    parsed
        .stacks
        .into_iter()
        .find(|stack| stack.branches.iter().any(|entry| entry.branch == branch))
        .map(|stack| {
            let branches = stack
                .branches
                .into_iter()
                .map(|entry| entry.branch)
                .collect();
            (stack.trunk.branch, branches)
        })
}

/// gh-stack keeps one state file per worktree, so a stack made in the main checkout or
/// another workspace is invisible here and the branch reads as unstacked. When another
/// worktree of the repo has a stack with this branch, set the same stack up here with
/// `gh stack init`. Returns whether the branch is stacked afterwards.
async fn adopt_local_stack(state: &AppState, worktree: &Path, branch: &str) -> bool {
    let Some((trunk, branches)) = stack_in_other_worktrees(state, worktree, branch).await else {
        return false;
    };
    // Merged branches may be gone locally, and `init` can't adopt a branch that isn't there.
    let mut present = Vec::with_capacity(branches.len());
    for name in branches {
        let local = format!("refs/heads/{name}");
        if state
            .git
            .ref_exists(worktree, &local)
            .await
            .unwrap_or(false)
        {
            present.push(name);
        }
    }
    let mut args = vec!["init", "--base", trunk.as_str()];
    args.extend(present.iter().map(String::as_str));
    // `init` saves the stack, then checks out its top branch. That fails when the branch
    // is checked out in the worktree the stack came from, so judge by `view`, not the exit code.
    if let Err(error) = gh_stack(state, worktree, &args).await {
        log::warn!("adopting the stack for {branch}: {error}");
        return false;
    }
    if state.git.current_branch(worktree).await.ok().as_deref() != Some(branch)
        && let Err(error) = state.git.switch(worktree, branch).await
    {
        log::warn!("switching back to {branch} after adopting its stack: {error}");
    }
    matches!(view(state, worktree).await, Ok(View::Stacked(_)))
}

async fn stack_in_other_worktrees(
    state: &AppState,
    worktree: &Path,
    branch: &str,
) -> Option<(String, Vec<String>)> {
    let (own, common) = match state.git.git_dirs(worktree).await {
        Ok(dirs) => dirs,
        Err(error) => {
            log::warn!("finding other worktrees' stacks: {error}");
            return None;
        }
    };

    let mut files = vec![common.join("gh-stack")];
    if let Ok(entries) = std::fs::read_dir(common.join("worktrees")) {
        files.extend(entries.flatten().map(|entry| entry.path().join("gh-stack")));
    }
    files
        .into_iter()
        .filter(|file| *file != own.join("gh-stack"))
        .filter_map(|file| std::fs::read_to_string(file).ok())
        .find_map(|text| stack_with_branch(&text, branch))
}

/// When GitHub last said each `worktree\0branch` has no stack. Asking means a network call on
/// every open of an unstacked branch, the common case; a stack made here shows up regardless,
/// since `view` is asked first.
#[derive(Default)]
pub struct RemoteStackCache {
    unstacked: Mutex<HashMap<String, Instant>>,
}

impl RemoteStackCache {
    fn key(worktree: &Path, branch: &str) -> String {
        format!("{}\0{branch}", worktree.display())
    }

    fn recently_unstacked(&self, worktree: &Path, branch: &str) -> bool {
        self.unstacked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&Self::key(worktree, branch))
            .is_some_and(|at| at.elapsed() < REMOTE_STACK_TTL)
    }

    fn remember_unstacked(&self, worktree: &Path, branch: &str) {
        let mut unstacked = self
            .unstacked
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        unstacked.retain(|_, at| at.elapsed() < REMOTE_STACK_TTL);
        unstacked.insert(Self::key(worktree, branch), Instant::now());
    }
}

enum RemoteStack {
    /// A stack on GitHub has the branch, and it's now set up in this worktree.
    CheckedOut,
    /// No stack on GitHub has the branch, or it's the trunk.
    None,
    /// Looking failed; the message says why, for the Stack tab to show.
    Failed(String),
}

/// A branch checked out from GitHub has no gh-stack metadata in this worktree, so
/// `gh stack view` calls it unstacked. `gh stack checkout <branch>` looks for it in the
/// stacks on GitHub too and, when one has it, sets that stack up here. This runs on loads of
/// an unstacked branch, so a stack made elsewhere shows up within `REMOTE_STACK_TTL` the next
/// time the Stack tab opens or the branch is checked out.
async fn checkout_remote_stack(
    state: &AppState,
    worktree: &Path,
    branch: &str,
    trunk: &str,
) -> RemoteStack {
    if branch.is_empty() || branch == trunk {
        return RemoteStack::None;
    }
    // With several remotes and no default, gh-stack asks which to fetch from, and there's
    // no terminal to answer. Cormux works against origin everywhere else, so default to it.
    let push_default = state
        .git
        .config_get(worktree, "remote.pushDefault")
        .await
        .unwrap_or_else(|error| {
            log::warn!("reading remote.pushDefault: {error}");
            None
        })
        .is_some();
    let origin_default: &[(&str, &str)] = if push_default {
        &[]
    } else {
        &[
            ("GIT_CONFIG_COUNT", "1"),
            ("GIT_CONFIG_KEY_0", "remote.pushDefault"),
            ("GIT_CONFIG_VALUE_0", "origin"),
        ]
    };

    let output =
        match gh_stack_with_env(state, worktree, &["checkout", branch], origin_default).await {
            Ok(output) => output,
            Err(Error::Github(message)) => return RemoteStack::Failed(message),
            Err(error) => return RemoteStack::Failed(error.to_string()),
        };
    match output.status.code() {
        Some(0) => {}
        Some(EXIT_NO_STACK) => return RemoteStack::None,
        // `checkout` uses gh-stack's conflict code when the stack here and the one on
        // GitHub hold different branches, not for a rebase.
        Some(EXIT_REBASE_CONFLICT) => {
            return RemoteStack::Failed(format!(
                "This branch's stack on GitHub has different branches from the one here. {}",
                stderr_text(&output)
            ));
        }
        _ => {
            let message = failure_message(&output, "Looking for this branch's stack on GitHub");
            log::warn!("gh stack checkout {branch}: {message}");
            return RemoteStack::Failed(message);
        }
    }
    // gh-stack checks the branch out as part of setting up the stack; keep the workspace
    // on the branch it asked for.
    if state.git.current_branch(worktree).await.ok().as_deref() != Some(branch)
        && let Err(error) = state.git.switch(worktree, branch).await
    {
        log::warn!("switching back to {branch} after gh stack checkout: {error}");
    }
    RemoteStack::CheckedOut
}

/// Add `branch` on top of the stack. When the checked-out branch isn't stacked yet,
/// start a stack first: on the trunk the new branch becomes its bottom, otherwise the
/// checked-out branch is adopted as the bottom and the new branch goes on top of it.
pub async fn add_branch(state: &AppState, workspace_id: &str, branch: &str) -> Result<()> {
    let name = branch.trim();
    let record = workspace_record(state, workspace_id).await?;
    let worktree = PathBuf::from(&record.worktree_path);
    refuse_new_branch(state, &record, &worktree, name).await?;
    state.workspaces.can_switch_branch(workspace_id).await?;

    match view(state, &worktree).await? {
        View::Unavailable(unavailable) => return Err(Error::Git(unavailable.text())),
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
        state,
        workspace_id,
        BranchChange::AddedToStack {
            on_top_of: record.branch.clone(),
        },
    )
    .await?;
    Ok(())
}

/// Why `name` can't be a new branch in the workspace, in words for the field it was typed in
/// (`Error::Invalid`).
async fn refuse_new_branch(
    state: &AppState,
    record: &WorkspaceRecord,
    worktree: &Path,
    name: &str,
) -> Result<()> {
    let refuse = |message: String| Err(Error::Invalid(message));
    if name.is_empty() {
        return refuse("Add a branch name, e.g. feat/oauth-login.".into());
    }
    if crate::git::validate_ref_name(name, "branch").is_err() {
        return refuse(
            "Use letters, numbers, dashes and slashes only, e.g. feat/oauth-login.".into(),
        );
    }
    let local = state.git.list_local_branches(worktree).await?;
    if name == record.base || local.iter().any(|branch| branch == name) {
        return refuse(format!("{name} already exists. Pick a new branch name."));
    }
    let taken = state
        .store
        .live_workspaces()?
        .into_iter()
        .any(|row| row.repo_id == record.repo_id && row.id != record.id && row.branch == name);
    if taken {
        return refuse("Another workspace already uses this branch.".into());
    }
    Ok(())
}

/// Push every unmerged branch in the stack (`--force-with-lease` per branch). Pull
/// requests stay manual: this never opens or edits one.
pub async fn push(state: &AppState, workspace_id: &str) -> Result<()> {
    let record = workspace_record(state, workspace_id).await?;
    let worktree = PathBuf::from(&record.worktree_path);
    let output = gh_stack(state, &worktree, &["push", "--remote", "origin"]).await?;
    if !output.status.success() {
        return Err(failure(&output, "Push"));
    }
    adopt_checked_out_branch(state, workspace_id, BranchChange::StackPushed).await?;
    toast(state, workspace_id, "Pushed the stack");
    Ok(())
}

/// Fetch, rebase onto the trunk, push, and refresh pull request state.
pub async fn sync(state: &AppState, workspace_id: &str) -> Result<()> {
    let record = workspace_record(state, workspace_id).await?;
    state.workspaces.can_switch_branch(workspace_id).await?;
    let worktree = PathBuf::from(&record.worktree_path);
    let output = gh_stack(state, &worktree, &["sync", "--remote", "origin"]).await?;
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
    adopt_checked_out_branch(state, workspace_id, BranchChange::StackSynced).await?;
    result?;
    toast(state, workspace_id, "Synced the stack with GitHub");
    Ok(())
}

fn sync_aborted(output: &CommandOutput) -> bool {
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    text.contains("Sync aborted")
}

async fn run(state: &AppState, worktree: &Path, args: &[&str], action: &str) -> Result<()> {
    let output = gh_stack(state, worktree, args).await?;
    if output.status.success() {
        Ok(())
    } else {
        Err(failure(&output, action))
    }
}

/// The branch a pull request for `branch` should target: the nearest unmerged branch
/// below it when it's stacked, otherwise `trunk`. A stack that can't be read is an error, not
/// "not stacked": guessing trunk would open a stacked branch's PR with the whole stack's diff.
pub async fn pr_base(
    state: &AppState,
    worktree: &Path,
    branch: &str,
    trunk: &str,
) -> Result<String> {
    let View::Stacked(json) = view(state, worktree).await? else {
        return Ok(trunk.to_string());
    };
    let parents = json.parents();
    Ok(json
        .branches
        .iter()
        .position(|row| row.name == branch && !row.is_merged)
        .map(|index| parents[index].clone())
        .unwrap_or_else(|| trunk.to_string()))
}

fn toast(state: &AppState, workspace_id: &str, text: &str) {
    state.out.toast(ToastRaisedPayload {
        tone: ToastTone::Ok,
        parts: vec![ToastPart::Text { value: text.into() }],
        workspace_id: Some(workspace_id.to_string()),
        thread_id: None,
    });
}

pub mod commands {
    use tauri::State;

    use crate::error::Result;
    use crate::state::AppState;
    use crate::workspace_git::commands::CreateWorkspaceBranchInput;

    use super::WorkspaceStack;

    #[tauri::command]
    #[specta::specta]
    pub async fn get_workspace_stack(
        workspace_id: String,
        state: State<'_, AppState>,
    ) -> Result<WorkspaceStack> {
        super::workspace_stack(&state, &workspace_id).await
    }

    #[tauri::command]
    #[specta::specta]
    pub async fn add_stack_branch(
        input: CreateWorkspaceBranchInput,
        state: State<'_, AppState>,
    ) -> Result<()> {
        super::add_branch(&state, &input.workspace_id, &input.branch).await
    }

    #[tauri::command]
    #[specta::specta]
    pub async fn push_stack(workspace_id: String, state: State<'_, AppState>) -> Result<()> {
        super::push(&state, &workspace_id).await
    }

    #[tauri::command]
    #[specta::specta]
    pub async fn sync_stack(workspace_id: String, state: State<'_, AppState>) -> Result<()> {
        super::sync(&state, &workspace_id).await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::Arc;

    use serde_json::{Value, json};
    use tempfile::TempDir;

    use super::*;
    use crate::model::WorkspaceLifecycle;
    use crate::shell_env::ShellEnv;
    use crate::store::types::{RepoRecord, WorkspaceRow};

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

    /// A `gh` that logs each call and answers `gh stack <subcommand>` with a canned reply.
    /// It goes on the PATH of the app's shell env, where every `gh` and `git` call looks,
    /// so this process's PATH is left alone.
    struct FakeGh {
        dir: TempDir,
    }

    impl FakeGh {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/stack/fake_gh.py");
            let shim = dir.path().join("gh");
            std::fs::write(
                &shim,
                format!(
                    "#!/bin/sh\nFAKE_GH_DIR='{}' exec python3 '{}' \"$@\"\n",
                    dir.path().display(),
                    script.display()
                ),
            )
            .unwrap();
            std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();
            std::fs::write(dir.path().join("replies.json"), "{}").unwrap();
            Self { dir }
        }

        fn env(&self) -> ShellEnv {
            let path = format!(
                "{}:{}",
                self.dir.path().display(),
                std::env::var("PATH").unwrap_or_default()
            );
            ShellEnv::from_vars([("PATH".to_string(), path)])
        }

        fn reply(&self, subcommand: &str, reply: Value) {
            let file = self.dir.path().join("replies.json");
            let mut replies: Value =
                serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
            replies[subcommand] = reply;
            std::fs::write(file, replies.to_string()).unwrap();
        }

        fn logged(&self) -> Vec<Value> {
            std::fs::read_to_string(self.dir.path().join("calls.jsonl"))
                .unwrap_or_default()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect()
        }

        /// Each call's argv, space-separated.
        fn calls(&self) -> Vec<String> {
            self.logged()
                .iter()
                .map(|call| {
                    let args: Vec<&str> = call["args"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|arg| arg.as_str().unwrap())
                        .collect();
                    args.join(" ")
                })
                .collect()
        }

        /// The GIT_CONFIG_* env of each call.
        fn git_config_env(&self) -> Vec<BTreeMap<String, String>> {
            self.logged()
                .into_iter()
                .map(|call| serde_json::from_value(call["env"].clone()).unwrap())
                .collect()
        }
    }

    fn exits(code: i32, stdout: &str, stderr: &str) -> Value {
        json!({"code": code, "stdout": stdout, "stderr": stderr})
    }

    /// Exits 0 after checking out `branch`, as gh-stack's `init` and `checkout` do.
    fn checks_out(branch: &str) -> Value {
        json!({"code": 0, "run": ["git", "switch", "-q", branch]})
    }

    /// A shell env whose PATH has `git` but no `gh`.
    fn env_without_gh() -> (TempDir, ShellEnv) {
        let dir = tempfile::tempdir().unwrap();
        let git = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .map(|bin| bin.join("git"))
            .find(|git| git.is_file())
            .expect("git on PATH");
        std::os::unix::fs::symlink(git, dir.path().join("git")).unwrap();
        let env = ShellEnv::from_vars([("PATH".to_string(), dir.path().display().to_string())]);
        (dir, env)
    }

    async fn state_on(env: ShellEnv) -> (AppState, Arc<crate::feedback::RecordingOutput>) {
        let (state, recording) =
            AppState::recorded(crate::paths::DataDir::new(std::env::temp_dir()));
        *state.shell_env.write().await = env;
        (state, recording)
    }

    fn git_cmd(cwd: &Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(["-c", "commit.gpgsign=false"])
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(output.status.success(), "git {args:?}: {output:?}");
    }

    /// A repo on `main` with branches `feat/a` and `feat/b`, and a worktree `wt` on `feat/a`.
    fn repo_and_worktree() -> (TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir(&repo).unwrap();
        git_cmd(&repo, &["init", "-q", "-b", "main"]);
        git_cmd(&repo, &["commit", "-q", "--allow-empty", "-m", "init"]);
        git_cmd(&repo, &["branch", "feat/a"]);
        git_cmd(&repo, &["branch", "feat/b"]);
        let worktree = dir.path().join("wt");
        git_cmd(
            &repo,
            &[
                "worktree",
                "add",
                "-q",
                worktree.to_str().unwrap(),
                "feat/a",
            ],
        );
        (dir, repo, worktree)
    }

    /// gh-stack's state file holding one stack.
    fn stack_file(trunk: &str, branches: &[&str]) -> String {
        let branches: Vec<Value> = branches
            .iter()
            .map(|branch| json!({"branch": branch, "base": "0"}))
            .collect();
        json!({
            "schemaVersion": 1,
            "repository": "",
            "stacks": [{"trunk": {"branch": trunk, "head": "0"}, "branches": branches}],
        })
        .to_string()
    }

    fn view_json(trunk: &str, current: &str, branches: &[&str]) -> String {
        let branches: Vec<Value> = branches
            .iter()
            .map(|name| json!({"name": name, "isCurrent": *name == current}))
            .collect();
        json!({"trunk": trunk, "currentBranch": current, "branches": branches}).to_string()
    }

    fn describe_view(view: Result<View>) -> String {
        match view {
            Ok(View::Stacked(_)) => "stacked".into(),
            Ok(View::NotStacked) => "not stacked".into(),
            Ok(View::Unavailable(unavailable)) => format!("unavailable: {}", unavailable.text()),
            Err(error) => format!("error: {error}"),
        }
    }

    fn describe_remote(remote: RemoteStack) -> String {
        match remote {
            RemoteStack::CheckedOut => "checked out".into(),
            RemoteStack::None => "none".into(),
            RemoteStack::Failed(message) => format!("failed: {message}"),
        }
    }

    const GH_MISSING: &str =
        "GitHub CLI not found. Install gh, then gh extension install github/gh-stack.";

    #[tokio::test]
    async fn view_reads_the_stack_from_gh_stack_view_json() {
        let fake = FakeGh::new();
        let (state, _) = state_on(fake.env()).await;
        let cwd = tempfile::tempdir().unwrap();
        fake.reply(
            "view",
            exits(
                0,
                r#"{
                  "trunk": "main",
                  "currentBranch": "b",
                  "branches": [
                    {"name": "a", "head": "1", "base": "0", "isCurrent": false, "isMerged": true,
                     "isQueued": false, "needsRebase": false,
                     "pr": {"number": 7, "url": "https://github.com/o/r/pull/7", "state": "MERGED"}},
                    {"name": "b", "base": "1", "isCurrent": true, "isMerged": false,
                     "isQueued": true, "needsRebase": true}
                  ]
                }"#,
                "",
            ),
        );

        let View::Stacked(json) = view(&state, cwd.path()).await.unwrap() else {
            panic!("not stacked");
        };
        assert_eq!(fake.calls(), ["stack view --json"]);
        assert_eq!(
            (json.trunk.as_str(), json.current_branch.as_str()),
            ("main", "b")
        );
        let [a, b] = &json.branches[..] else {
            panic!("{:?}", json.branches);
        };
        assert_eq!(a.name, "a");
        assert!(a.is_merged && !a.is_queued && !a.needs_rebase);
        let pr = a.pr.as_ref().unwrap();
        assert_eq!(
            (pr.number, pr.url.as_deref(), pr.state.as_str()),
            (7, Some("https://github.com/o/r/pull/7"), "MERGED")
        );
        assert_eq!(b.name, "b");
        assert!(!b.is_merged && b.is_queued && b.needs_rebase);
        assert!(b.pr.is_none());
        assert_eq!(json.parents(), ["main", "main"]);
    }

    #[tokio::test]
    async fn a_new_stack_branch_name_is_checked_by_the_core() {
        use crate::model::WorkspaceLifecycle;
        use crate::store::types::WorkspaceRow;

        let (state, _) = state_on(ShellEnv::new()).await;
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        let git = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(repo)
                .status()
                .unwrap();
            assert!(status.success(), "git {args:?}");
        };
        git(&["init", "-q", "-b", "main"]);
        git(&[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "a",
        ]);
        git(&["branch", "feat/old"]);
        state
            .store
            .upsert_repo(&crate::store::fixtures::repo("r1"))
            .unwrap();
        for (id, branch) in [("w1", "feat/a"), ("w2", "feat/b")] {
            let row = WorkspaceRow {
                branch: branch.into(),
                worktree_path: repo.to_string_lossy().into(),
                ..crate::store::fixtures::workspace(id, WorkspaceLifecycle::Idle)
            };
            state.store.insert_workspace(&row).unwrap();
        }
        let record = state.workspaces.get("w1").await.unwrap();

        for (name, refusal) in [
            ("", "Add a branch name"),
            ("bad name", "Use letters, numbers, dashes and slashes only"),
            ("main", "main already exists"),
            ("feat/old", "feat/old already exists"),
            ("feat/b", "Another workspace already uses this branch"),
        ] {
            let error = refuse_new_branch(&state, &record, repo, name)
                .await
                .unwrap_err();
            assert!(
                matches!(&error, Error::Invalid(text) if text.contains(refusal)),
                "{name:?}: {error}"
            );
        }
        refuse_new_branch(&state, &record, repo, "feat/new")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn view_says_why_a_branch_has_no_stack() {
        let fake = FakeGh::new();
        let (state, _) = state_on(fake.env()).await;
        let cwd = tempfile::tempdir().unwrap();
        let cases = [
            (exits(2, "", "not in a stack\n"), "not stacked"),
            (
                exits(9, "", ""),
                "unavailable: Stacked pull requests aren't enabled on this repository.",
            ),
            (
                exits(1, "", "unknown command \"stack\" for \"gh\"\n"),
                "unavailable: The gh-stack extension isn't installed. Run: gh extension install github/gh-stack",
            ),
            (
                exits(1, "", "✗ not a git repository\n"),
                "error: git: Reading the stack failed: not a git repository",
            ),
            (
                exits(0, "not json", ""),
                "error: git: could not read gh stack view --json: expected ident at line 1 column 2",
            ),
        ];
        for (reply, expected) in cases {
            fake.reply("view", reply);
            assert_eq!(describe_view(view(&state, cwd.path()).await), expected);
        }
        assert!(fake.calls().iter().all(|call| call == "stack view --json"));
        // The panel shows the install command on its own, not parsed out of the reason.
        fake.reply(
            "view",
            exits(1, "", "unknown command \"stack\" for \"gh\"\n"),
        );
        let Ok(View::Unavailable(missing)) = view(&state, cwd.path()).await else {
            panic!("not unavailable");
        };
        assert_eq!(missing.install, Some(INSTALL_HINT));
        assert!(!missing.reason.contains(INSTALL_HINT));

        let (_bin, env) = env_without_gh();
        let (state, _) = state_on(env).await;
        assert_eq!(
            describe_view(view(&state, cwd.path()).await),
            format!("unavailable: {GH_MISSING}")
        );
    }

    /// Bug: any failure to read the stack (gh-stack busy, bad output) targeted trunk, so a
    /// stacked branch's PR opened against main with the whole stack's diff.
    #[tokio::test]
    async fn a_pr_targets_trunk_only_when_the_branch_is_not_stacked() {
        let fake = FakeGh::new();
        let (state, _) = state_on(fake.env()).await;
        let cwd = tempfile::tempdir().unwrap();
        let base = |state: AppState, cwd: PathBuf| async move {
            pr_base(&state, &cwd, "b", "main")
                .await
                .map_err(|error| error.to_string())
        };

        fake.reply("view", exits(0, &view_json("main", "b", &["a", "b"]), ""));
        assert_eq!(base(state.clone(), cwd.path().into()).await.unwrap(), "a");
        fake.reply("view", exits(2, "", "not in a stack\n"));
        assert_eq!(
            base(state.clone(), cwd.path().into()).await.unwrap(),
            "main"
        );
        fake.reply("view", exits(EXIT_LOCKED, "", "locked\n"));
        let error = base(state.clone(), cwd.path().into()).await.unwrap_err();
        assert!(
            error.contains("waiting on another gh stack command"),
            "{error}"
        );
        fake.reply("view", exits(0, "not json", ""));
        assert!(base(state.clone(), cwd.path().into()).await.is_err());
    }

    #[tokio::test]
    async fn a_failed_gh_stack_command_explains_itself() {
        let fake = FakeGh::new();
        let (state, _) = state_on(fake.env()).await;
        let cwd = tempfile::tempdir().unwrap();
        let cases = [
            (
                exits(3, "", "conflict in a.txt\n"),
                "Adding the branch stopped on a rebase conflict and put the stack back. Run `gh stack rebase` in the worktree to resolve it.",
            ),
            (
                exits(4, "", "HTTP 401: Bad credentials\n"),
                "Adding the branch failed talking to GitHub. Check `gh auth status`, then retry. HTTP 401: Bad credentials",
            ),
            (
                exits(8, "", ""),
                "Adding the branch is waiting on another gh stack command. Try again in a few seconds.",
            ),
            (
                exits(9, "", ""),
                "Stacked pull requests aren't enabled on this repository.",
            ),
            (
                exits(
                    1,
                    "",
                    "gh stack is available as an official extension.\nTo install it, run:\n  gh extension install github/gh-stack\n",
                ),
                "gh-stack isn't installed. Run: gh extension install github/gh-stack",
            ),
            (
                exits(1, "", "✗ cannot add\n! the branch exists\n"),
                "Adding the branch failed: cannot add the branch exists",
            ),
            (
                exits(1, "✗ said on stdout\n", ""),
                "Adding the branch failed: said on stdout",
            ),
            (exits(1, "", ""), "Adding the branch failed"),
        ];
        for (reply, expected) in cases {
            fake.reply("add", reply);
            let error = run(&state, cwd.path(), &["add", "feat/c"], "Adding the branch")
                .await
                .unwrap_err();
            assert_eq!(error.to_string(), format!("git: {expected}"));
        }
        fake.reply("add", exits(0, "", ""));
        run(&state, cwd.path(), &["add", "feat/c"], "Adding the branch")
            .await
            .unwrap();
        assert!(fake.calls().iter().all(|call| call == "stack add feat/c"));

        let (_bin, env) = env_without_gh();
        let (state, _) = state_on(env).await;
        let error = run(&state, cwd.path(), &["add", "feat/c"], "Adding the branch")
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), format!("github: {GH_MISSING}"));
    }

    #[tokio::test]
    async fn sync_reports_failures_and_divergence_and_toasts_success() {
        let fake = FakeGh::new();
        let (state, recording) = state_on(fake.env()).await;
        let (_dir, repo, worktree) = repo_and_worktree();
        state
            .store
            .upsert_repo(&RepoRecord {
                path: repo.to_string_lossy().to_string(),
                ..crate::store::fixtures::repo("r1")
            })
            .unwrap();
        state
            .store
            .insert_workspace(&WorkspaceRow {
                branch: "feat/a".into(),
                worktree_path: worktree.to_string_lossy().to_string(),
                ..crate::store::fixtures::workspace("w", WorkspaceLifecycle::Ready)
            })
            .unwrap();

        let diverged = "git: Sync stopped: the stack on GitHub and this worktree have diverged. Run `gh stack sync` in a terminal to choose which to keep.";
        let cases = [
            (exits(0, "Sync aborted: branches diverged\n", ""), diverged),
            (exits(0, "", "! Sync aborted\n"), diverged),
            (
                exits(3, "", ""),
                "git: Sync stopped on a rebase conflict and put the stack back. Run `gh stack rebase` in the worktree to resolve it.",
            ),
        ];
        for (reply, expected) in cases {
            fake.reply("sync", reply);
            let error = sync(&state, "w").await.unwrap_err();
            assert_eq!(error.to_string(), expected);
        }
        assert!(recording.toasts().is_empty());

        fake.reply("sync", exits(0, "✓ Synced\n", ""));
        sync(&state, "w").await.unwrap();
        assert_eq!(recording.toasts(), ["Synced the stack with GitHub"]);
        assert_eq!(fake.calls(), vec!["stack sync --remote origin"; 4]);
    }

    /// A stack made in the main checkout shows up in a workspace on one of its branches.
    #[tokio::test]
    async fn adopts_a_stack_made_in_another_worktree() {
        let fake = FakeGh::new();
        let (state, _) = state_on(fake.env()).await;
        let (_dir, repo, worktree) = repo_and_worktree();
        std::fs::write(
            repo.join(".git/gh-stack"),
            stack_file("main", &["feat/gone", "feat/a", "feat/b"]),
        )
        .unwrap();
        fake.reply("init", checks_out("feat/b"));
        fake.reply(
            "view",
            exits(0, &view_json("main", "feat/a", &["feat/a", "feat/b"]), ""),
        );

        assert!(adopt_local_stack(&state, &worktree, "feat/a").await);
        // The merged branch that's gone locally is left out of `init`.
        assert_eq!(
            fake.calls(),
            ["stack init --base main feat/a feat/b", "stack view --json"]
        );
        assert_eq!(state.git.current_branch(&worktree).await.unwrap(), "feat/a");
    }

    /// `init` fails when the top branch is checked out where the stack came from, but the
    /// stack is saved by then: `view` decides.
    #[tokio::test]
    async fn adopting_goes_by_view_not_by_inits_exit_code() {
        let fake = FakeGh::new();
        let (state, _) = state_on(fake.env()).await;
        let (dir, repo, worktree) = repo_and_worktree();
        let other = dir.path().join("other");
        git_cmd(
            &repo,
            &["worktree", "add", "-q", other.to_str().unwrap(), "feat/b"],
        );
        std::fs::write(
            repo.join(".git/worktrees/other/gh-stack"),
            stack_file("main", &["feat/a", "feat/b"]),
        )
        .unwrap();
        fake.reply(
            "init",
            exits(1, "", "✗ feat/b is already checked out at other\n"),
        );

        fake.reply(
            "view",
            exits(0, &view_json("main", "feat/a", &["feat/a", "feat/b"]), ""),
        );
        assert!(adopt_local_stack(&state, &worktree, "feat/a").await);

        fake.reply("view", exits(2, "", ""));
        assert!(!adopt_local_stack(&state, &worktree, "feat/a").await);

        assert_eq!(
            fake.calls(),
            [
                "stack init --base main feat/a feat/b",
                "stack view --json",
                "stack init --base main feat/a feat/b",
                "stack view --json",
            ]
        );
    }

    #[tokio::test]
    async fn a_branch_no_other_worktree_stacks_is_left_alone() {
        let fake = FakeGh::new();
        let (state, _) = state_on(fake.env()).await;
        let (_dir, repo, worktree) = repo_and_worktree();
        assert!(!adopt_local_stack(&state, &worktree, "feat/a").await);

        // Another worktree's stack without the branch, and this worktree's own file.
        std::fs::write(repo.join(".git/gh-stack"), stack_file("main", &["feat/b"])).unwrap();
        std::fs::write(
            repo.join(".git/worktrees/wt/gh-stack"),
            stack_file("main", &["feat/a"]),
        )
        .unwrap();
        assert!(!adopt_local_stack(&state, &worktree, "feat/a").await);

        assert!(fake.calls().is_empty(), "{:?}", fake.calls());
    }

    #[tokio::test]
    async fn checking_out_a_remote_stack_keeps_the_workspace_on_its_branch() {
        let fake = FakeGh::new();
        let (state, _) = state_on(fake.env()).await;
        let (_dir, repo, worktree) = repo_and_worktree();
        fake.reply("checkout", checks_out("feat/b"));

        let remote = checkout_remote_stack(&state, &worktree, "feat/a", "main").await;
        assert_eq!(describe_remote(remote), "checked out");
        assert_eq!(state.git.current_branch(&worktree).await.unwrap(), "feat/a");

        // gh-stack can't ask which remote to use, so it's told origin unless the repo says.
        git_cmd(&repo, &["config", "remote.pushDefault", "upstream"]);
        let remote = checkout_remote_stack(&state, &worktree, "feat/a", "main").await;
        assert_eq!(describe_remote(remote), "checked out");

        assert_eq!(
            fake.calls(),
            ["stack checkout feat/a", "stack checkout feat/a"]
        );
        let origin = BTreeMap::from([
            ("GIT_CONFIG_COUNT".to_string(), "1".to_string()),
            (
                "GIT_CONFIG_KEY_0".to_string(),
                "remote.pushDefault".to_string(),
            ),
            ("GIT_CONFIG_VALUE_0".to_string(), "origin".to_string()),
        ]);
        assert_eq!(fake.git_config_env(), [origin, BTreeMap::new()]);
    }

    #[tokio::test]
    async fn the_trunk_and_a_detached_head_are_not_looked_up() {
        let fake = FakeGh::new();
        let (state, _) = state_on(fake.env()).await;
        let (_dir, _repo, worktree) = repo_and_worktree();
        for branch in ["main", ""] {
            let remote = checkout_remote_stack(&state, &worktree, branch, "main").await;
            assert_eq!(describe_remote(remote), "none");
        }
        assert!(fake.calls().is_empty(), "{:?}", fake.calls());
    }

    #[tokio::test]
    async fn a_remote_stack_lookup_says_what_went_wrong() {
        let fake = FakeGh::new();
        let (state, _) = state_on(fake.env()).await;
        let (_dir, _repo, worktree) = repo_and_worktree();
        let cases = [
            (exits(2, "", "no stack has feat/a\n"), "none".to_string()),
            (
                exits(3, "", "✗ the local stack has feat/x\n"),
                "failed: This branch's stack on GitHub has different branches from the one here. the local stack has feat/x".into(),
            ),
            (
                exits(4, "", "HTTP 502\n"),
                "failed: Looking for this branch's stack on GitHub failed talking to GitHub. Check `gh auth status`, then retry. HTTP 502".into(),
            ),
            (
                exits(1, "", "unknown command \"stack\" for \"gh\"\n"),
                "failed: gh-stack isn't installed. Run: gh extension install github/gh-stack"
                    .into(),
            ),
            (
                exits(1, "", ""),
                "failed: Looking for this branch's stack on GitHub failed".into(),
            ),
        ];
        for (reply, expected) in cases {
            fake.reply("checkout", reply);
            let remote = checkout_remote_stack(&state, &worktree, "feat/a", "main").await;
            assert_eq!(describe_remote(remote), expected);
        }
        assert!(
            fake.calls()
                .iter()
                .all(|call| call == "stack checkout feat/a")
        );
        assert_eq!(state.git.current_branch(&worktree).await.unwrap(), "feat/a");

        let (_bin, env) = env_without_gh();
        let (state, _) = state_on(env).await;
        let remote = checkout_remote_stack(&state, &worktree, "feat/a", "main").await;
        assert_eq!(describe_remote(remote), format!("failed: {GH_MISSING}"));
    }

    #[test]
    fn finds_the_stack_with_a_branch_in_a_stack_file() {
        let file = r#"{
          "schemaVersion": 1,
          "repository": "",
          "stacks": [
            {"trunk": {"branch": "main", "head": "0"},
             "branches": [{"branch": "x", "base": "0"}]},
            {"trunk": {"branch": "develop", "head": "1"},
             "branches": [{"branch": "a", "base": "1"}, {"branch": "b", "base": "2"}]}
          ]
        }"#;
        assert_eq!(
            stack_with_branch(file, "b"),
            Some(("develop".into(), vec!["a".into(), "b".into()]))
        );
        assert_eq!(stack_with_branch(file, "nope"), None);
        assert_eq!(stack_with_branch("not json", "b"), None);
    }
}
