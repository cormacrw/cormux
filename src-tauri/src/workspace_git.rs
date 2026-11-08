//! Git state per workspace: how far it is behind its base, any conflict a pull stopped on, and
//! every way its branch moves.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::sync::RwLock;

use crate::error::{Error, Result};
use crate::feedback::{ToastPart, ToastRaisedPayload, ToastTone};
use crate::git::{BehindUpdate, GitConflictState, RepoFetchTarget, WorkspaceFetchTarget};
use crate::state::AppState;
use crate::timeline::{BranchChange, TimelineStep};
use crate::workspaces::WorkspaceRecord;

const FETCH_INTERVAL: Duration = Duration::from_secs(300);
const BRANCH_WATCH_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Default)]
struct Stats {
    behind: u32,
    conflict: Option<GitConflictState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceGitRuntime {
    pub workspace_id: String,
    pub behind: u32,
    pub conflict: Option<GitConflictState>,
}

/// How a new worktree gets its branch.
#[derive(Debug, Clone, Copy)]
pub enum Checkout {
    /// The workspace's branch, made from the latest base unless it already exists.
    NewBranch,
    /// A pull request's head, fetched from its `pull/<n>/head` ref.
    PullRef(u64),
}

#[derive(Clone, Default)]
pub struct WorkspaceGit {
    stats: Arc<RwLock<HashMap<String, Stats>>>,
}

impl WorkspaceGit {
    /// Waits for the lock: a `try_read` here gave an empty map whenever a count was being
    /// written, so a snapshot taken mid-pull showed every workspace level and conflict-free.
    pub async fn snapshot(&self, workspace_ids: &[String]) -> Vec<WorkspaceGitRuntime> {
        let stats = self.stats.read().await;
        workspace_ids
            .iter()
            .map(|id| {
                let git = stats.get(id).cloned().unwrap_or_default();
                WorkspaceGitRuntime {
                    workspace_id: id.clone(),
                    behind: git.behind,
                    conflict: git.conflict,
                }
            })
            .collect()
    }

    pub async fn forget(&self, workspace_id: &str) {
        self.stats.write().await.remove(workspace_id);
    }

    async fn get(&self, workspace_id: &str) -> Stats {
        self.stats
            .read()
            .await
            .get(workspace_id)
            .cloned()
            .unwrap_or_default()
    }

    async fn patch(&self, workspace_id: &str, patch: impl FnOnce(&mut Stats)) {
        let mut stats = self.stats.write().await;
        patch(stats.entry(workspace_id.to_string()).or_default());
    }

    async fn apply(&self, updates: &[BehindUpdate]) {
        let mut stats = self.stats.write().await;
        for update in updates {
            stats.entry(update.workspace_id.clone()).or_default().behind = update.behind;
        }
    }
}

pub fn start(state: &AppState) {
    let fetching = state.clone();
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(FETCH_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            fetch_all(&fetching).await;
        }
    });

    // The diff watcher only runs for the workspace on screen, so a checkout from a terminal or
    // by an agent in any other worktree would go unnoticed. Reading `HEAD` costs no process.
    let watching = state.clone();
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(BRANCH_WATCH_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            adopt_outside_checkouts(&watching).await;
        }
    });
}

pub async fn fetch_all(state: &AppState) {
    let targets = match fetch_targets(state).await {
        Ok(targets) => targets,
        Err(error) => {
            log::warn!("listing workspaces to fetch: {error}");
            return;
        }
    };
    let updates = state.fetch.tick(targets).await;
    state.workspace_git.apply(&updates).await;
    state.out.repo_git_moved();
}

async fn fetch_targets(state: &AppState) -> Result<Vec<RepoFetchTarget>> {
    let mut by_repo: HashMap<String, RepoFetchTarget> = HashMap::new();
    for workspace in state.workspaces.live().await? {
        let worktree = workspace.worktree();
        if !worktree.is_dir() {
            continue;
        }
        by_repo
            .entry(workspace.repo_path.clone())
            .or_insert_with(|| RepoFetchTarget {
                repo_path: PathBuf::from(&workspace.repo_path),
                workspaces: Vec::new(),
            })
            .workspaces
            .push(WorkspaceFetchTarget {
                workspace_id: workspace.id,
                worktree_path: worktree,
                base: workspace.base,
            });
    }
    Ok(by_repo.into_values().collect())
}

pub async fn add_worktree(
    state: &AppState,
    workspace: &WorkspaceRecord,
    checkout: Checkout,
) -> Result<()> {
    let repo = PathBuf::from(&workspace.repo_path);
    let worktree = workspace.worktree();
    match checkout {
        Checkout::PullRef(number) => {
            state
                .git
                .fetch_pull_ref(&repo, number, &workspace.branch)
                .await?;
            if !worktree.exists() {
                state
                    .git
                    .worktree_add_detached(&repo, &worktree, &workspace.branch)
                    .await?;
            }
        }
        Checkout::NewBranch => {
            // Continuing a PR reuses its branch: check it out (git tracks origin's copy when
            // there is no local one) rather than failing on `-b`.
            let exists = state
                .git
                .list_local_branches(&repo)
                .await?
                .contains(&workspace.branch)
                || state
                    .git
                    .remote_branch_exists(&repo, &workspace.branch)
                    .await?;
            if exists {
                state
                    .git
                    .worktree_add_detached(&repo, &worktree, &workspace.branch)
                    .await?;
            } else {
                let start = state.git.latest_base(&repo, &workspace.base).await;
                state
                    .git
                    .worktree_add(&repo, &worktree, &workspace.branch, &start)
                    .await?;
            }
        }
    }
    Ok(())
}

pub async fn refresh_stats(state: &AppState, workspace: &WorkspaceRecord) {
    let worktree = workspace.worktree();
    let behind = state
        .git
        .behind_count(&worktree, &workspace.base)
        .await
        .unwrap_or_else(|error| {
            log::warn!("behind count for {}: {error}", workspace.id);
            0
        });
    state
        .workspace_git
        .patch(&workspace.id, |stats| stats.behind = behind)
        .await;
}

/// The PR on record was the old branch's, so it is cleared.
async fn on_branch_changed(
    state: &AppState,
    workspace: &WorkspaceRecord,
    branch: &str,
    change: BranchChange,
) -> Result<()> {
    let updated = state.workspaces.set_branch(&workspace.id, branch).await?;
    state.workspaces.set_pr(&workspace.id, None, None)?;
    refresh_stats(state, &updated).await;
    refresh_diff(state, &updated).await;
    record_in_own_thread(
        state,
        &workspace.id,
        TimelineStep::BranchSwitched {
            branch: branch.to_string(),
            change,
        },
    );
    toast(
        state,
        &workspace.id,
        vec![
            text("Switched "),
            code(&workspace.name),
            text(" to "),
            code(branch),
        ],
    );
    state.out.repo_git_moved();
    Ok(())
}

pub async fn switch_branch(state: &AppState, workspace_id: &str, branch: &str) -> Result<()> {
    crate::git::validate_ref_name(branch, "branch")?;
    let workspace = state.workspaces.get(workspace_id).await?;
    if workspace.branch == branch {
        return Ok(());
    }
    state.workspaces.can_switch_branch(workspace_id).await?;
    ensure_not_default_branch(state, &workspace.repo_id, branch)?;
    state.git.switch(&workspace.worktree(), branch).await?;
    let change = BranchChange::CheckedOut {
        previous: workspace.branch.clone(),
    };
    on_branch_changed(state, &workspace, branch, change).await
}

pub async fn create_branch(state: &AppState, workspace_id: &str, branch: &str) -> Result<()> {
    let branch = branch.trim();
    if branch.is_empty() {
        return Err(Error::Git("branch name is required".into()));
    }
    let workspace = state.workspaces.get(workspace_id).await?;
    state.workspaces.can_switch_branch(workspace_id).await?;
    ensure_not_default_branch(state, &workspace.repo_id, branch)?;
    state
        .git
        .switch_new_branch(&workspace.worktree(), branch, "HEAD")
        .await?;
    let change = BranchChange::Created {
        previous: workspace.branch.clone(),
    };
    on_branch_changed(state, &workspace, branch, change).await
}

/// Brings the workspace in line with whatever branch an outside tool (`gh stack`, a terminal)
/// left checked out, and refreshes its counts and diff. Returns the branch.
pub async fn adopt_checked_out_branch(
    state: &AppState,
    workspace_id: &str,
    change: BranchChange,
) -> Result<String> {
    let workspace = state.workspaces.get(workspace_id).await?;
    let branch = state.git.current_branch(&workspace.worktree()).await?;
    // Empty while HEAD is detached (mid-rebase): the record keeps its branch.
    if !branch.is_empty() && branch != workspace.branch {
        on_branch_changed(state, &workspace, &branch, change).await?;
        return Ok(branch);
    }
    refresh_stats(state, &workspace).await;
    refresh_diff(state, &workspace).await;
    state.out.repo_git_moved();
    Ok(workspace.branch)
}

async fn adopt_outside_checkouts(state: &AppState) {
    let workspaces = match state.workspaces.live().await {
        Ok(workspaces) => workspaces,
        Err(error) => {
            log::warn!("branch watch: {error}");
            return;
        }
    };
    for workspace in workspaces {
        if !workspace.status.is_settled() {
            continue;
        }
        let worktree = workspace.worktree();
        if !worktree.exists() {
            continue;
        }
        let branch = match crate::git::checked_out_branch(&worktree) {
            Some(branch) => branch,
            None => match state.git.current_branch(&worktree).await {
                Ok(branch) => branch,
                Err(_) => continue,
            },
        };
        if branch.is_empty() || branch == workspace.branch {
            continue;
        }
        let change = BranchChange::CheckedOutOutside {
            previous: workspace.branch.clone(),
        };
        if let Err(error) = on_branch_changed(state, &workspace, &branch, change).await {
            log::warn!(
                "branch watch: adopting {branch} in {}: {error}",
                workspace.id
            );
        }
    }
}

/// Merges the latest base in. A conflict is kept for the header to resolve or abort.
pub async fn pull(state: &AppState, workspace_id: &str) -> Result<()> {
    let workspace = state.workspaces.get(workspace_id).await?;
    let (base, branch) = (workspace.base.clone(), workspace.branch.clone());
    let worktree = workspace.worktree();
    // Counted now, after a fetch: the count the header shows can be minutes old.
    state.git.latest_base(&worktree, &base).await;
    let behind = state.git.behind_count(&worktree, &base).await?;
    if behind == 0 {
        return Err(Error::Git(format!("already up to date with {base}")));
    }
    match state.git.merge_base(&worktree, &base).await {
        Ok(()) => {}
        Err(Error::GitConflict { paths }) => {
            state
                .workspace_git
                .patch(workspace_id, |stats| {
                    stats.conflict = Some(GitConflictState { paths });
                })
                .await;
            state.out.repo_git_moved();
            return Err(Error::Git(
                "merge stopped with conflicts — resolve or abort from the workspace header".into(),
            ));
        }
        Err(error) => return Err(error),
    }

    state
        .workspace_git
        .patch(workspace_id, |stats| stats.conflict = None)
        .await;
    refresh_stats(state, &workspace).await;
    refresh_diff(state, &workspace).await;
    record_in_own_thread(
        state,
        workspace_id,
        TimelineStep::CommitsPulled {
            count: behind,
            base: base.clone(),
            branch,
        },
    );
    toast(
        state,
        workspace_id,
        vec![
            text("Pulled "),
            code(&behind.to_string()),
            text(&format!(
                " commit{} from ",
                if behind == 1 { "" } else { "s" }
            )),
            code(&base),
        ],
    );
    state.out.repo_git_moved();
    Ok(())
}

pub async fn abort_conflict(state: &AppState, workspace_id: &str) -> Result<()> {
    let workspace = state.workspaces.get(workspace_id).await?;
    if state
        .workspace_git
        .get(workspace_id)
        .await
        .conflict
        .is_none()
    {
        return Ok(());
    }
    state.git.abort_merge(&workspace.worktree()).await?;
    state
        .workspace_git
        .patch(workspace_id, |stats| stats.conflict = None)
        .await;
    refresh_stats(state, &workspace).await;
    refresh_diff(state, &workspace).await;
    state.out.repo_git_moved();
    Ok(())
}

fn ensure_not_default_branch(state: &AppState, repo_id: &str, branch: &str) -> Result<()> {
    match state.store.repo_by_id(repo_id)? {
        Some(repo) => repo.ensure_not_default_branch(branch),
        None => Ok(()),
    }
}

async fn refresh_diff(state: &AppState, workspace: &WorkspaceRecord) {
    if let Err(error) = state
        .diffs
        .compute(&workspace.id, Path::new(&workspace.worktree_path))
        .await
    {
        log::warn!("diff refresh for {}: {error}", workspace.id);
    }
}

/// Git steps go in the workspace's own thread, its Lead or Reviewer.
pub fn record_in_own_thread(state: &AppState, workspace_id: &str, step: TimelineStep) {
    let recorded = state
        .store
        .own_thread(workspace_id)
        .and_then(|thread| match thread {
            Some(thread) => state.transcript.record_step(&thread.id, step).map(|_| ()),
            None => Ok(()),
        });
    if let Err(error) = recorded {
        log::warn!("recording a git step in {workspace_id}: {error}");
    }
}

fn toast(state: &AppState, workspace_id: &str, parts: Vec<ToastPart>) {
    state.out.toast(ToastRaisedPayload {
        tone: ToastTone::Ok,
        parts,
        workspace_id: Some(workspace_id.to_string()),
        thread_id: None,
    });
}

fn text(value: &str) -> ToastPart {
    ToastPart::Text {
        value: value.to_string(),
    }
}

fn code(value: &str) -> ToastPart {
    ToastPart::Code {
        value: value.to_string(),
    }
}

pub mod commands {
    use serde::{Deserialize, Serialize};
    use specta::Type;
    use tauri::State;

    use crate::error::Result;
    use crate::state::AppState;

    #[derive(Debug, Clone, Serialize, Deserialize, Type)]
    #[serde(rename_all = "camelCase")]
    pub struct SwitchWorkspaceBranchInput {
        pub workspace_id: String,
        pub branch: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Type)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateWorkspaceBranchInput {
        pub workspace_id: String,
        pub branch: String,
    }

    #[tauri::command]
    #[specta::specta]
    pub async fn switch_workspace_branch(
        input: SwitchWorkspaceBranchInput,
        state: State<'_, AppState>,
    ) -> Result<()> {
        super::switch_branch(&state, &input.workspace_id, &input.branch).await
    }

    #[tauri::command]
    #[specta::specta]
    pub async fn pull_workspace(workspace_id: String, state: State<'_, AppState>) -> Result<()> {
        super::pull(&state, &workspace_id).await
    }

    #[tauri::command]
    #[specta::specta]
    pub async fn abort_workspace_git(
        workspace_id: String,
        state: State<'_, AppState>,
    ) -> Result<()> {
        super::abort_conflict(&state, &workspace_id).await
    }

    #[tauri::command]
    #[specta::specta]
    pub async fn create_workspace_branch(
        input: CreateWorkspaceBranchInput,
        state: State<'_, AppState>,
    ) -> Result<()> {
        super::create_branch(&state, &input.workspace_id, &input.branch).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ThreadRole, ThreadStatus, WorkspaceKind};
    use crate::shell_env::ShellEnv;
    use crate::store::types::{RepoRecord, ThreadRow, WorkspaceRow};

    fn git(cwd: &Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(output.status.success(), "git {args:?}: {output:?}");
    }

    fn add_repo(state: &AppState, path: &Path) {
        state
            .store
            .upsert_repo(&RepoRecord {
                path: path.to_string_lossy().to_string(),
                name: "app".into(),
                ..crate::store::fixtures::repo("r1")
            })
            .unwrap();
    }

    /// The background fetch used to throw its counts away, so they only moved on focus.
    #[tokio::test]
    async fn a_background_fetch_updates_how_far_each_workspace_is_behind() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        git(root, &["init", "-q", "--bare", "-b", "main", "origin.git"]);
        git(root, &["clone", "-q", "origin.git", "seed"]);
        let seed = root.join("seed");
        git(&seed, &["commit", "-q", "--allow-empty", "-m", "one"]);
        git(&seed, &["push", "-q", "origin", "HEAD:main"]);
        git(root, &["clone", "-q", "origin.git", "app"]);
        let app = root.join("app");
        let worktree = root.join("wt");
        git(
            &app,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "feat/x",
                &worktree.to_string_lossy(),
                "origin/main",
            ],
        );
        git(&seed, &["commit", "-q", "--allow-empty", "-m", "two"]);
        git(&seed, &["push", "-q", "origin", "HEAD:main"]);

        let (state, _recording) = AppState::recorded(crate::paths::DataDir::new(root.join("data")));
        *state.shell_env.write().await =
            ShellEnv::from_vars([("PATH".to_string(), "/usr/bin:/bin".to_string())]);
        add_repo(&state, &app);
        state
            .store
            .insert_workspace(&WorkspaceRow::new(
                WorkspaceKind::Lead,
                "w",
                "r1",
                "Work",
                "feat/x",
                "main",
                worktree.to_string_lossy(),
            ))
            .unwrap();

        fetch_all(&state).await;

        let runtime = state.workspace_git.snapshot(&["w".to_string()]).await;
        assert_eq!(runtime[0].behind, 1);
    }

    /// A review workspace has no Lead; its git steps, like the PR it opens, go to its Reviewer.
    #[tokio::test]
    async fn a_review_workspaces_git_steps_go_to_its_reviewer() {
        let (state, _recording) =
            AppState::recorded(crate::paths::DataDir::new(std::env::temp_dir()));
        add_repo(&state, Path::new("/tmp/app"));
        state
            .store
            .insert_workspace(&WorkspaceRow::new(
                WorkspaceKind::PrReview,
                "pr",
                "r1",
                "PR #7 Review",
                "fix/x",
                "main",
                "/tmp/pr",
            ))
            .unwrap();
        state
            .store
            .upsert_thread(
                &ThreadRow::new("rt", "pr", ThreadRole::Reviewer, "claude")
                    .with_status(ThreadStatus::Idle),
            )
            .unwrap();

        record_in_own_thread(
            &state,
            "pr",
            TimelineStep::PrOpened {
                number: 9,
                branch: "fix/x".into(),
                base: "main".into(),
            },
        );

        let events = state
            .store
            .thread_event_page("rt", None, None, None)
            .unwrap()
            .events;
        assert_eq!(events.len(), 1);
        assert!(
            events[0].payload.contains("prOpened"),
            "{}",
            events[0].payload
        );
    }
}
