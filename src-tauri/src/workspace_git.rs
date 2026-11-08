//! Git state per workspace: how far it is behind its base, any conflict a pull stopped on, and
//! every way its branch moves.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::sync::RwLock;

use crate::error::{Error, Result};
use crate::feedback::{ToastPart, ToastTone};
use crate::git::{BehindUpdate, GitConflictState, RepoFetchTarget, WorkspaceFetchTarget};
use crate::repos::{BranchHold, BranchHolds};
use crate::state::AppState;
use crate::store::types::RepoRecord;
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

#[derive(Debug, Clone, Copy, Default)]
struct BranchMoves {
    held: usize,
    started: u64,
}

type MovesByWorkspace = HashMap<String, BranchMoves>;

/// Held by a Cormux operation that can move a workspace's HEAD, for as long as it can.
pub struct BranchMoveGuard {
    moves: Arc<Mutex<MovesByWorkspace>>,
    workspace_id: String,
}

impl Drop for BranchMoveGuard {
    fn drop(&mut self) {
        let mut moves = self.moves.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(entry) = moves.get_mut(&self.workspace_id) {
            entry.held = entry.held.saturating_sub(1);
        }
    }
}

#[derive(Clone, Default)]
pub struct WorkspaceGit {
    stats: Arc<RwLock<HashMap<String, Stats>>>,
    /// Kept after the last guard drops: the count started is how the branch watcher sees an
    /// operation that came and went between its reads.
    moves: Arc<Mutex<MovesByWorkspace>>,
}

impl WorkspaceGit {
    /// Cormux's own branch operations check branches out and back as separate git calls, and
    /// adopt what they leave themselves; the branch watcher keeps out of a workspace while one
    /// is held.
    pub fn hold_branch(&self, workspace_id: &str) -> BranchMoveGuard {
        let mut moves = self.moves.lock().unwrap_or_else(PoisonError::into_inner);
        let entry = moves.entry(workspace_id.to_string()).or_default();
        entry.held += 1;
        entry.started += 1;
        BranchMoveGuard {
            moves: Arc::clone(&self.moves),
            workspace_id: workspace_id.to_string(),
        }
    }

    fn branch_moves(&self) -> MovesByWorkspace {
        self.moves
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Whether a branch operation in `workspace_id` is held now or started after `before`.
    fn branch_moved_since(&self, before: &MovesByWorkspace, workspace_id: &str) -> bool {
        let now = self
            .moves
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(workspace_id)
            .copied()
            .unwrap_or_default();
        let started_before = before.get(workspace_id).map_or(0, |moves| moves.started);
        now.held > 0 || now.started != started_before
    }

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
        self.moves
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(workspace_id);
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
                let start = state.git.latest_base(&repo, &workspace.base).await?;
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
    let behind = state.git.behind_count(&worktree, &workspace.base).await;
    let conflict = state.git.merge_conflict(&worktree).await;
    state
        .workspace_git
        .patch(&workspace.id, |stats| {
            // A count git can't make leaves the last one standing rather than reading as none.
            match behind {
                Ok(behind) => stats.behind = behind,
                Err(error) => log::warn!("behind count for {}: {error}", workspace.id),
            }
            match conflict {
                Ok(conflict) => stats.conflict = conflict,
                Err(error) => log::warn!("merge conflict for {}: {error}", workspace.id),
            }
        })
        .await;
}

/// Git owns whether a merge is stopped on conflicts: one finished or aborted outside Cormux
/// clears, and one still stopped after a restart shows again.
async fn sync_conflict(state: &AppState, workspace: &WorkspaceRecord) {
    let worktree = workspace.worktree();
    let known = state.workspace_git.get(&workspace.id).await.conflict;
    if known.is_none() && !crate::git::merging(&worktree) {
        return;
    }
    let conflict = match state.git.merge_conflict(&worktree).await {
        Ok(conflict) => conflict,
        Err(error) => {
            log::warn!("merge conflict for {}: {error}", workspace.id);
            return;
        }
    };
    if conflict != known {
        state
            .workspace_git
            .patch(&workspace.id, |stats| stats.conflict = conflict)
            .await;
        state.out.repo_git_moved();
    }
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
            ToastPart::text("Switched "),
            ToastPart::code(&workspace.name),
            ToastPart::text(" to "),
            ToastPart::code(branch),
        ],
    );
    state.out.repo_git_moved();
    Ok(())
}

/// Whether a workspace can have `name` as its branch at all: not its repo's default, which
/// stays in the repo checkout, and a name git takes. Refusals are `Error::Invalid`, worded for
/// the field the name was typed in.
pub fn check_branch_name(repo: &RepoRecord, name: &str) -> Result<()> {
    ensure_named(name)?;
    BranchHolds::of_repo(repo.clone()).ensure_not_default(name)?;
    ensure_ref_name(name)
}

fn ensure_named(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        return Err(Error::Invalid(
            "Add a branch name, e.g. feat/oauth-login.".into(),
        ));
    }
    Ok(())
}

fn ensure_ref_name(name: &str) -> Result<()> {
    crate::git::validate_ref_name(name, "branch").map_err(|_| {
        Error::Invalid(
            "Use letters, numbers, dashes and slashes only, e.g. feat/oauth-login.".into(),
        )
    })
}

/// Whether `name` can be made as a new branch in `workspace`: a name a workspace can have,
/// that no branch in the repo has yet and no other live workspace uses.
pub async fn check_new_branch(
    state: &AppState,
    workspace: &WorkspaceRecord,
    name: &str,
) -> Result<()> {
    ensure_named(name)?;
    ensure_ref_name(name)?;
    let local = state.git.list_local_branches(&workspace.worktree()).await?;
    if name == workspace.base || local.iter().any(|branch| branch == name) {
        return Err(Error::Invalid(format!(
            "{name} already exists. Pick a new branch name."
        )));
    }
    let holds = BranchHolds::recorded(state, state.store.repo(&workspace.repo_id)?)?
        .seen_from(&workspace.id);
    if let Some(BranchHold::CheckedOutBy { .. }) = holds.of(name) {
        return Err(Error::Invalid(
            "Another workspace already uses this branch.".into(),
        ));
    }
    holds.ensure_not_default(name)
}

/// Every branch name a new workspace in the repo shouldn't draft: local and `origin`'s
/// branches, and those live workspaces (other than `except_workspace`) use.
pub async fn taken_branches(
    state: &AppState,
    repo_id: &str,
    repo_path: &Path,
    except_workspace: Option<&str>,
) -> Result<Vec<String>> {
    let mut taken = crate::repos::branch_names(state, repo_path).await?;
    let mut holds = BranchHolds::recorded(state, state.store.repo(repo_id)?)?;
    if let Some(workspace_id) = except_workspace {
        holds = holds.seen_from(workspace_id);
    }
    taken.extend(holds.checked_out_names().map(str::to_string));
    Ok(taken)
}

pub async fn switch_branch(state: &AppState, workspace_id: &str, branch: &str) -> Result<()> {
    let workspace = state.workspaces.get(workspace_id).await?;
    check_branch_name(&state.store.repo(&workspace.repo_id)?, branch)?;
    if workspace.branch == branch {
        return Ok(());
    }
    state.workspaces.can_switch_branch(workspace_id).await?;
    let _moving = state.workspace_git.hold_branch(workspace_id);
    state.git.switch(&workspace.worktree(), branch).await?;
    let change = BranchChange::CheckedOut {
        previous: workspace.branch.clone(),
    };
    on_branch_changed(state, &workspace, branch, change).await
}

pub async fn create_branch(state: &AppState, workspace_id: &str, branch: &str) -> Result<()> {
    let branch = branch.trim();
    let workspace = state.workspaces.get(workspace_id).await?;
    check_new_branch(state, &workspace, branch).await?;
    state.workspaces.can_switch_branch(workspace_id).await?;
    let _moving = state.workspace_git.hold_branch(workspace_id);
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
    // Read before the records, so an operation that switched and recorded in between counts.
    let moves = state.workspace_git.branch_moves();
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
        sync_conflict(state, &workspace).await;
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
        // Checked after reading HEAD: a branch read mid-operation may be gone already.
        if state
            .workspace_git
            .branch_moved_since(&moves, &workspace.id)
        {
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
    state.git.latest_base(&worktree, &base).await?;
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
            ToastPart::text("Pulled "),
            ToastPart::code(behind.to_string()),
            ToastPart::text(format!(
                " commit{} from ",
                if behind == 1 { "" } else { "s" }
            )),
            ToastPart::code(&base),
        ],
    );
    state.out.repo_git_moved();
    Ok(())
}

pub async fn abort_conflict(state: &AppState, workspace_id: &str) -> Result<()> {
    let workspace = state.workspaces.get(workspace_id).await?;
    state.git.abort_merge(&workspace.worktree()).await?;
    refresh_stats(state, &workspace).await;
    refresh_diff(state, &workspace).await;
    state.out.repo_git_moved();
    Ok(())
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
    state
        .out
        .toast_parts(ToastTone::Ok, parts, Some(workspace_id.to_string()));
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

    /// A repo on `feat/a` with `f.txt` committed, its own checkout the worktree of a settled
    /// workspace `w` with PR #7.
    async fn workspace_in_a_repo(root: &Path) -> (AppState, PathBuf) {
        let repo = root.join("app");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-q", "-b", "main"]);
        std::fs::write(repo.join("f.txt"), "base\n").unwrap();
        git(&repo, &["add", "f.txt"]);
        git(&repo, &["commit", "-q", "-m", "base"]);
        git(&repo, &["switch", "-q", "-c", "feat/a"]);
        let (state, _recording) = AppState::recorded(crate::paths::DataDir::new(root.join("data")));
        state
            .shell_env
            .replace(ShellEnv::from_vars([(
                "PATH".to_string(),
                "/usr/bin:/bin".to_string(),
            )]))
            .await;
        add_repo(&state, &repo);
        let row = WorkspaceRow::new(
            WorkspaceKind::Lead,
            "w",
            "r1",
            "Work",
            "feat/a",
            "main",
            repo.to_string_lossy(),
        )
        .with_status(crate::model::WorkspaceLifecycle::Ready)
        .with_pr(7, None);
        state.store.insert_workspace(&row).unwrap();
        (state, repo)
    }

    /// `main` and `feat/a` each change `f.txt`, and `feat/a` merges `main`, stopping on it.
    fn stop_a_merge_on_a_conflict(repo: &Path) {
        git(repo, &["switch", "-q", "main"]);
        std::fs::write(repo.join("f.txt"), "main\n").unwrap();
        git(repo, &["commit", "-q", "-am", "main"]);
        git(repo, &["switch", "-q", "feat/a"]);
        std::fs::write(repo.join("f.txt"), "feat\n").unwrap();
        git(repo, &["commit", "-q", "-am", "feat"]);
        merge_main_into_a_conflict(repo);
    }

    fn merge_main_into_a_conflict(repo: &Path) {
        let merge = std::process::Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@t", "merge", "main"])
            .current_dir(repo)
            .output()
            .unwrap();
        assert!(!merge.status.success(), "{merge:?}");
    }

    async fn conflict(state: &AppState) -> Option<GitConflictState> {
        state.workspace_git.snapshot(&["w".to_string()]).await[0]
            .conflict
            .clone()
    }

    /// Bug: a merge's conflict lived only in memory, so its banner stayed after the merge was
    /// finished or aborted outside Cormux, Abort then failed with "There is no merge to abort",
    /// and a restart forgot a merge still stopped. Git is asked now.
    #[tokio::test]
    async fn a_merge_conflict_is_whatever_git_says_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let (state, repo) = workspace_in_a_repo(dir.path()).await;
        stop_a_merge_on_a_conflict(&repo);

        // Stopped before this state ever pulled, as after a restart.
        adopt_outside_checkouts(&state).await;
        let stopped = Some(GitConflictState {
            paths: vec!["f.txt".into()],
        });
        assert_eq!(conflict(&state).await, stopped);

        git(&repo, &["merge", "--abort"]);
        abort_conflict(&state, "w").await.unwrap();
        assert_eq!(conflict(&state).await, None);

        merge_main_into_a_conflict(&repo);
        adopt_outside_checkouts(&state).await;
        assert_eq!(conflict(&state).await, stopped);
        std::fs::write(repo.join("f.txt"), "both\n").unwrap();
        git(&repo, &["commit", "-q", "-am", "resolved"]);
        adopt_outside_checkouts(&state).await;
        assert_eq!(conflict(&state).await, None);
    }

    /// Bug: the branch watcher took a branch a stack operation checked out for a moment as one
    /// checked out outside Cormux, recording a switch and dropping the workspace's PR.
    #[tokio::test]
    async fn the_branch_watcher_keeps_out_while_cormux_moves_a_branch() {
        let dir = tempfile::tempdir().unwrap();
        let (state, repo) = workspace_in_a_repo(dir.path()).await;
        git(&repo, &["branch", "feat/b"]);
        let pr = |state: &AppState| state.store.workspace_by_id("w").unwrap().unwrap().pr_number;

        let moving = state.workspace_git.hold_branch("w");
        git(&repo, &["switch", "-q", "feat/b"]);
        adopt_outside_checkouts(&state).await;
        assert_eq!(state.workspaces.get("w").await.unwrap().branch, "feat/a");
        assert_eq!(pr(&state), Some(7));
        git(&repo, &["switch", "-q", "feat/a"]);
        drop(moving);

        git(&repo, &["switch", "-q", "feat/b"]);
        adopt_outside_checkouts(&state).await;
        assert_eq!(state.workspaces.get("w").await.unwrap().branch, "feat/b");
        assert_eq!(pr(&state), None);
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
        state
            .shell_env
            .replace(ShellEnv::from_vars([(
                "PATH".to_string(),
                "/usr/bin:/bin".to_string(),
            )]))
            .await;
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

    /// Bugs: the header's new branch took a name another workspace uses, and the stack's took
    /// the repo's default branch when the workspace was based on another. Both now share one
    /// check, worded for the field.
    #[tokio::test]
    async fn a_new_branch_name_is_checked_the_same_way_everywhere() {
        let (state, _recording) =
            AppState::recorded(crate::paths::DataDir::new(std::env::temp_dir()));
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        git(repo, &["init", "-q", "-b", "main"]);
        git(repo, &["commit", "-q", "--allow-empty", "-m", "a"]);
        git(repo, &["branch", "feat/old"]);
        state
            .store
            .upsert_repo(&RepoRecord {
                default_branch: "trunk".into(),
                ..crate::store::fixtures::repo("r1")
            })
            .unwrap();
        for (id, branch) in [("w1", "feat/a"), ("w2", "feat/b")] {
            let row = WorkspaceRow {
                branch: branch.into(),
                base: "main".into(),
                worktree_path: repo.to_string_lossy().into(),
                ..crate::store::fixtures::workspace(id, crate::model::WorkspaceLifecycle::Idle)
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
            ("trunk", "the default branch"),
        ] {
            let error = check_new_branch(&state, &record, name).await.unwrap_err();
            assert!(
                matches!(&error, Error::Invalid(text) if text.contains(refusal)),
                "{name:?}: {error}"
            );
        }
        check_new_branch(&state, &record, "feat/new").await.unwrap();

        let error = create_branch(&state, "w1", "feat/b").await.unwrap_err();
        assert!(matches!(error, Error::Invalid(_)), "{error}");
    }

    /// Bug: a behind count git couldn't make read as 0, so the header said the branch was up to
    /// date with its base.
    #[tokio::test]
    async fn a_behind_count_git_cant_make_keeps_the_last_one() {
        let root = tempfile::tempdir().unwrap();
        let (state, repo) = workspace_in_a_repo(root.path()).await;
        git(&repo, &["switch", "-q", "main"]);
        std::fs::write(repo.join("g.txt"), "later\n").unwrap();
        git(&repo, &["add", "g.txt"]);
        git(&repo, &["commit", "-q", "-m", "later"]);
        git(&repo, &["switch", "-q", "feat/a"]);
        let record = state.workspaces.get("w").await.unwrap();
        let behind = |state: AppState| async move {
            state.workspace_git.snapshot(&["w".to_string()]).await[0].behind
        };

        refresh_stats(&state, &record).await;
        assert_eq!(behind(state.clone()).await, 1);

        std::fs::remove_dir_all(repo.join(".git")).unwrap();
        refresh_stats(&state, &record).await;
        assert_eq!(behind(state.clone()).await, 1);
    }
}
