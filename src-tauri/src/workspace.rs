use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::git::{FetchScheduler, Git, LiveDiffEngine, RepoFetchTarget, WorkspaceFetchTarget};
use crate::ipc::types::GitConflictState;

/// Workspace lifecycle. `ready` is idle with no threads; running/idle/waiting
/// describe activity after provisioning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceLifecycle {
    Creating,
    Provisioning,
    Ready,
    Running,
    Idle,
    Waiting,
    TearingDown,
    Gone,
    ProvisioningFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ThreadActivity {
    Provisioning,
    Running,
    Paused,
    Idle,
    Waiting,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRecord {
    pub id: String,
    pub repo_id: String,
    pub repo_path: String,
    pub name: String,
    pub branch: String,
    pub base: String,
    pub worktree_path: String,
    pub status: WorkspaceLifecycle,
    pub version: u64,
    pub activity: String,
    pub prov_step: u8,
    pub setup_failed_command: Option<String>,
    pub setup_failed_exit_code: Option<i32>,
}

#[derive(Debug, Clone)]
struct ThreadSlot {
    workspace_id: String,
    activity: ThreadActivity,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceGitStats {
    pub behind: u32,
    pub ahead: u32,
    pub conflict: Option<GitConflictState>,
}

struct Inner {
    workspaces: HashMap<String, WorkspaceRecord>,
    threads: HashMap<String, ThreadSlot>,
    git_stats: HashMap<String, WorkspaceGitStats>,
}

/// Lifecycle state machine plus branch-switch lock.
#[derive(Clone)]
pub struct WorkspaceManager {
    git: Git,
    diffs: LiveDiffEngine,
    fetch: FetchScheduler,
    inner: Arc<RwLock<Inner>>,
}

impl WorkspaceManager {
    pub fn new(git: Git, diffs: LiveDiffEngine, fetch: FetchScheduler) -> Self {
        Self {
            git,
            diffs,
            fetch,
            inner: Arc::new(RwLock::new(Inner {
                workspaces: HashMap::new(),
                threads: HashMap::new(),
                git_stats: HashMap::new(),
            })),
        }
    }

    pub fn default_worktrees_base(home: &Path) -> PathBuf {
        home.join(".harness").join("worktrees")
    }

    pub fn worktree_path(worktrees_base: &Path, repo: &str, branch: &str) -> PathBuf {
        worktrees_base.join(repo).join(branch_slug(branch))
    }

    pub fn harness_env(workspace: &WorkspaceRecord) -> Vec<(String, String)> {
        vec![
            ("HARNESS_REPO_PATH".into(), workspace.repo_path.clone()),
            (
                "HARNESS_WORKTREE_PATH".into(),
                workspace.worktree_path.clone(),
            ),
            ("HARNESS_WORKSPACE".into(), workspace.name.clone()),
            ("HARNESS_BRANCH".into(), workspace.branch.clone()),
            ("HARNESS_BASE".into(), workspace.base.clone()),
        ]
    }

    pub async fn list(&self) -> Vec<WorkspaceRecord> {
        self.inner
            .read()
            .await
            .workspaces
            .values()
            .cloned()
            .collect()
    }

    pub async fn get(&self, id: &str) -> Option<WorkspaceRecord> {
        self.inner.read().await.workspaces.get(id).cloned()
    }

    pub async fn remember(&self, record: WorkspaceRecord) {
        self.inner
            .write()
            .await
            .workspaces
            .insert(record.id.clone(), record);
    }

    pub async fn rename(&self, workspace_id: &str, name: &str) -> Result<WorkspaceRecord> {
        let mut inner = self.inner.write().await;
        let workspace = inner
            .workspaces
            .get_mut(workspace_id)
            .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
        workspace.name = name.to_string();
        workspace.version += 1;
        Ok(workspace.clone())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn register_provisioning(
        &self,
        workspace_id: &str,
        repo_id: &str,
        repo_path: &Path,
        name: &str,
        branch: &str,
        base: &str,
        worktrees_base: &Path,
    ) -> Result<WorkspaceRecord> {
        let repo_name = repo_dir_name(repo_path);
        let worktree = Self::worktree_path(worktrees_base, repo_name, branch);
        if let Some(parent) = worktree.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let record = WorkspaceRecord {
            id: workspace_id.to_string(),
            repo_id: repo_id.to_string(),
            repo_path: repo_path.to_string_lossy().to_string(),
            name: name.to_string(),
            branch: branch.to_string(),
            base: base.to_string(),
            worktree_path: worktree.to_string_lossy().to_string(),
            status: WorkspaceLifecycle::Provisioning,
            version: 1,
            activity: "Running worktree setup…".into(),
            prov_step: 1,
            setup_failed_command: None,
            setup_failed_exit_code: None,
        };
        self.inner
            .write()
            .await
            .workspaces
            .insert(workspace_id.to_string(), record.clone());
        Ok(record)
    }

    pub async fn add_review_worktree(
        &self,
        workspace_id: &str,
        pr_number: u64,
    ) -> Result<WorkspaceRecord> {
        let workspace = self
            .get(workspace_id)
            .await
            .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
        let repo_path = PathBuf::from(&workspace.repo_path);
        let worktree = PathBuf::from(&workspace.worktree_path);
        self.git
            .fetch_pull_ref(&repo_path, pr_number, &workspace.branch)
            .await?;
        if !worktree.exists() {
            self.git
                .worktree_add_detached(&repo_path, &worktree, &workspace.branch)
                .await?;
        }
        self.diffs.watch(workspace_id, &worktree)?;
        self.sync_fetch_targets().await;
        self.get(workspace_id)
            .await
            .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))
    }

    pub async fn add_worktree(&self, workspace_id: &str) -> Result<WorkspaceRecord> {
        let workspace = self
            .get(workspace_id)
            .await
            .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
        let repo_path = PathBuf::from(&workspace.repo_path);
        let worktree = PathBuf::from(&workspace.worktree_path);
        // Continuing a PR reuses its branch: check it out (git tracks origin's copy
        // when there is no local one) rather than failing on `-b`.
        let branch_exists = self
            .git
            .list_local_branches(&repo_path)
            .await?
            .contains(&workspace.branch)
            || self
                .git
                .remote_branch_exists(&repo_path, &workspace.branch)
                .await?;
        if branch_exists {
            self.git
                .worktree_add_detached(&repo_path, &worktree, &workspace.branch)
                .await?;
        } else {
            self.git
                .worktree_add(&repo_path, &worktree, &workspace.branch, &workspace.base)
                .await?;
        }
        self.diffs.watch(workspace_id, &worktree)?;
        self.sync_fetch_targets().await;
        self.get(workspace_id)
            .await
            .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))
    }

    pub async fn set_provisioning_detail(
        &self,
        workspace_id: &str,
        activity: &str,
        prov_step: u8,
        failed_command: Option<String>,
        exit_code: Option<i32>,
    ) -> Result<WorkspaceRecord> {
        let mut inner = self.inner.write().await;
        let workspace = inner
            .workspaces
            .get_mut(workspace_id)
            .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
        workspace.activity = activity.to_string();
        workspace.prov_step = prov_step;
        workspace.setup_failed_command = failed_command;
        workspace.setup_failed_exit_code = exit_code;
        workspace.version += 1;
        Ok(workspace.clone())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        repo_id: &str,
        repo_path: &Path,
        name: &str,
        branch: &str,
        base: &str,
        worktrees_base: &Path,
        workspace_id: Option<&str>,
    ) -> Result<WorkspaceRecord> {
        let id = workspace_id
            .map(str::to_string)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        self.register_provisioning(&id, repo_id, repo_path, name, branch, base, worktrees_base)
            .await?;
        self.add_worktree(&id).await?;
        self.set_status(&id, WorkspaceLifecycle::Ready).await
    }

    pub async fn set_status(
        &self,
        id: &str,
        status: WorkspaceLifecycle,
    ) -> Result<WorkspaceRecord> {
        let mut inner = self.inner.write().await;
        let workspace = inner
            .workspaces
            .get_mut(id)
            .ok_or_else(|| Error::Workspace(format!("unknown workspace {id}")))?;
        assert_transition(workspace.status, status)?;
        workspace.status = status;
        workspace.version += 1;
        Ok(workspace.clone())
    }

    pub async fn set_thread(&self, thread_id: &str, workspace_id: &str, activity: ThreadActivity) {
        let mut inner = self.inner.write().await;
        inner.threads.insert(
            thread_id.to_string(),
            ThreadSlot {
                workspace_id: workspace_id.to_string(),
                activity,
            },
        );
        let activities: Vec<ThreadActivity> = inner
            .threads
            .values()
            .filter(|thread| thread.workspace_id == workspace_id)
            .map(|thread| thread.activity)
            .collect();
        if let Some(workspace) = inner.workspaces.get_mut(workspace_id) {
            workspace.status = status_from_threads(workspace.status, activities.into_iter());
            workspace.version += 1;
        }
    }

    /// Forget a closed thread so it no longer counts toward the workspace's status.
    pub async fn remove_thread(&self, thread_id: &str, workspace_id: &str) {
        let mut inner = self.inner.write().await;
        inner.threads.remove(thread_id);
        let activities: Vec<ThreadActivity> = inner
            .threads
            .values()
            .filter(|thread| thread.workspace_id == workspace_id)
            .map(|thread| thread.activity)
            .collect();
        if let Some(workspace) = inner.workspaces.get_mut(workspace_id) {
            workspace.status = status_from_threads(workspace.status, activities.into_iter());
            workspace.version += 1;
        }
    }

    pub async fn can_switch_branch(&self, workspace_id: &str) -> Result<()> {
        let inner = self.inner.read().await;
        let workspace = inner
            .workspaces
            .get(workspace_id)
            .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
        if matches!(
            workspace.status,
            WorkspaceLifecycle::Creating | WorkspaceLifecycle::Provisioning
        ) {
            return Err(Error::Workspace(
                "branch switch refused while provisioning".into(),
            ));
        }
        let blocked = inner.threads.values().any(|thread| {
            thread.workspace_id == workspace_id
                && matches!(
                    thread.activity,
                    ThreadActivity::Running | ThreadActivity::Provisioning
                )
        });
        if blocked {
            return Err(Error::Workspace(
                "branch switch refused while a thread is running".into(),
            ));
        }
        Ok(())
    }

    pub async fn switch_branch_record(&self, workspace_id: &str, branch: &str) -> Result<()> {
        let mut inner = self.inner.write().await;
        let workspace = inner
            .workspaces
            .get_mut(workspace_id)
            .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
        workspace.branch = branch.to_string();
        workspace.version += 1;
        let stats = inner.git_stats.entry(workspace_id.to_string()).or_default();
        stats.behind = 0;
        Ok(())
    }

    pub fn git_stats_snapshot(&self) -> HashMap<String, WorkspaceGitStats> {
        match self.inner.try_read() {
            Ok(inner) => inner.git_stats.clone(),
            Err(_) => HashMap::new(),
        }
    }

    pub async fn git_stats(&self, workspace_id: &str) -> WorkspaceGitStats {
        self.inner
            .read()
            .await
            .git_stats
            .get(workspace_id)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn patch_git_stats(
        &self,
        workspace_id: &str,
        patch: impl FnOnce(&mut WorkspaceGitStats),
    ) {
        let mut inner = self.inner.write().await;
        let stats = inner.git_stats.entry(workspace_id.to_string()).or_default();
        patch(stats);
    }

    pub async fn set_git_conflict(&self, workspace_id: &str, conflict: GitConflictState) {
        let mut inner = self.inner.write().await;
        let stats = inner.git_stats.entry(workspace_id.to_string()).or_default();
        stats.conflict = Some(conflict);
    }

    pub async fn clear_git_conflict(&self, workspace_id: &str) {
        let mut inner = self.inner.write().await;
        if let Some(stats) = inner.git_stats.get_mut(workspace_id) {
            stats.conflict = None;
        }
    }

    pub async fn remove(&self, workspace_id: &str) {
        let mut inner = self.inner.write().await;
        inner.workspaces.remove(workspace_id);
        inner
            .threads
            .retain(|_, slot| slot.workspace_id != workspace_id);
    }

    pub async fn sync_fetch_targets(&self) {
        let inner = self.inner.read().await;
        let mut by_repo: HashMap<String, RepoFetchTarget> = HashMap::new();
        for workspace in inner.workspaces.values() {
            if workspace.status == WorkspaceLifecycle::Gone {
                continue;
            }
            let entry = by_repo
                .entry(workspace.repo_path.clone())
                .or_insert_with(|| RepoFetchTarget {
                    repo_path: PathBuf::from(&workspace.repo_path),
                    workspaces: Vec::new(),
                });
            entry.workspaces.push(WorkspaceFetchTarget {
                workspace_id: workspace.id.clone(),
                worktree_path: PathBuf::from(&workspace.worktree_path),
                base: workspace.base.clone(),
            });
        }
        drop(inner);
        self.fetch.set_repos(by_repo.into_values().collect()).await;
    }
}

fn branch_slug(branch: &str) -> String {
    branch.replace('/', "-")
}

fn repo_dir_name(repo_path: &Path) -> &str {
    repo_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo")
}

fn status_from_threads(
    current: WorkspaceLifecycle,
    threads: impl Iterator<Item = ThreadActivity>,
) -> WorkspaceLifecycle {
    if matches!(
        current,
        WorkspaceLifecycle::Creating
            | WorkspaceLifecycle::Provisioning
            | WorkspaceLifecycle::TearingDown
            | WorkspaceLifecycle::Gone
            | WorkspaceLifecycle::ProvisioningFailed
    ) {
        return current;
    }
    let threads: Vec<_> = threads.collect();
    if threads.iter().any(|activity| {
        matches!(
            activity,
            ThreadActivity::Running | ThreadActivity::Provisioning
        )
    }) {
        WorkspaceLifecycle::Running
    } else if threads.contains(&ThreadActivity::Waiting) {
        WorkspaceLifecycle::Waiting
    } else if threads.is_empty() {
        WorkspaceLifecycle::Ready
    } else {
        WorkspaceLifecycle::Idle
    }
}

fn assert_transition(from: WorkspaceLifecycle, to: WorkspaceLifecycle) -> Result<()> {
    if from == to {
        return Ok(());
    }
    let ok = matches!(
        (from, to),
        (
            WorkspaceLifecycle::Creating,
            WorkspaceLifecycle::Provisioning
        ) | (
            WorkspaceLifecycle::Provisioning,
            WorkspaceLifecycle::Ready
                | WorkspaceLifecycle::Running
                | WorkspaceLifecycle::ProvisioningFailed
        ) | (
            WorkspaceLifecycle::Ready,
            WorkspaceLifecycle::Running
                | WorkspaceLifecycle::Idle
                | WorkspaceLifecycle::Waiting
                | WorkspaceLifecycle::TearingDown
        ) | (
            WorkspaceLifecycle::Running | WorkspaceLifecycle::Idle | WorkspaceLifecycle::Waiting,
            WorkspaceLifecycle::Ready
                | WorkspaceLifecycle::Running
                | WorkspaceLifecycle::Idle
                | WorkspaceLifecycle::Waiting
                | WorkspaceLifecycle::TearingDown
        ) | (
            WorkspaceLifecycle::ProvisioningFailed,
            WorkspaceLifecycle::Provisioning | WorkspaceLifecycle::TearingDown
        ) | (WorkspaceLifecycle::TearingDown, WorkspaceLifecycle::Gone)
    );
    if ok {
        Ok(())
    } else {
        Err(Error::Workspace(format!(
            "illegal transition {from:?} → {to:?}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::Git;
    use crate::shell_env::ShellEnv;

    async fn manager() -> WorkspaceManager {
        let mut env = ShellEnv::new();
        env.load_or_inherit().await.unwrap();
        let git = Git::new(Arc::new(tokio::sync::RwLock::new(env)));
        let diffs = LiveDiffEngine::new(git.clone());
        let fetch = FetchScheduler::new(git.clone());
        WorkspaceManager::new(git, diffs, fetch)
    }

    fn init_repo() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_path_buf();
        let run = |args: &[&str]| {
            assert!(
                std::process::Command::new(args[0])
                    .args(&args[1..])
                    .current_dir(&path)
                    .status()
                    .unwrap()
                    .success()
            );
        };
        run(&["git", "init", "-b", "main"]);
        run(&["git", "config", "user.email", "cormux@test"]);
        run(&["git", "config", "user.name", "Cormux"]);
        std::fs::write(path.join("README.md"), "hello\n").unwrap();
        run(&["git", "add", "README.md"]);
        run(&["git", "commit", "-m", "init"]);
        (dir, path)
    }

    #[test]
    fn worktree_path_includes_repo_and_branch_slug() {
        let base = Path::new("/Users/dev/.harness/worktrees");
        let path = WorkspaceManager::worktree_path(base, "my-app", "feat/login");
        assert_eq!(
            path,
            PathBuf::from("/Users/dev/.harness/worktrees/my-app/feat-login")
        );
    }

    #[tokio::test]
    async fn lock_blocks_switch_while_running_and_allows_when_idle() {
        let mgr = manager().await;
        let (_dir, repo) = init_repo();
        let home = tempfile::tempdir().unwrap();
        let workspace = mgr
            .create(
                "repo-1",
                &repo,
                "Login",
                "feat-login",
                "main",
                home.path(),
                None,
            )
            .await
            .unwrap();
        assert_eq!(workspace.status, WorkspaceLifecycle::Ready);
        assert!(workspace.version >= 2);

        mgr.set_thread("t1", &workspace.id, ThreadActivity::Running)
            .await;
        assert!(mgr.can_switch_branch(&workspace.id).await.is_err());

        mgr.set_thread("t1", &workspace.id, ThreadActivity::Idle)
            .await;
        mgr.can_switch_branch(&workspace.id).await.unwrap();

        let env = WorkspaceManager::harness_env(&workspace);
        assert!(env.iter().any(|(key, _)| key == "HARNESS_WORKTREE_PATH"));
    }

    #[test]
    fn rejects_illegal_transitions() {
        assert!(assert_transition(WorkspaceLifecycle::Gone, WorkspaceLifecycle::Ready).is_err());
        assert!(
            assert_transition(
                WorkspaceLifecycle::Creating,
                WorkspaceLifecycle::Provisioning
            )
            .is_ok()
        );
    }
}
