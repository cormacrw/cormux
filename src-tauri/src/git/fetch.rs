use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::sync::RwLock;
use tokio::task::JoinSet;

use crate::git::Git;

const FETCH_INTERVAL: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BehindUpdate {
    pub workspace_id: String,
    pub behind: u32,
}

#[derive(Debug, Clone)]
pub struct WorkspaceFetchTarget {
    pub workspace_id: String,
    pub worktree_path: PathBuf,
    pub base: String,
}

#[derive(Debug, Clone)]
pub struct RepoFetchTarget {
    pub repo_path: PathBuf,
    pub workspaces: Vec<WorkspaceFetchTarget>,
}

/// One `git fetch` per repo (worktrees share an object store), then behind counts.
#[derive(Clone)]
pub struct FetchScheduler {
    git: Git,
    repos: Arc<RwLock<Vec<RepoFetchTarget>>>,
}

impl FetchScheduler {
    pub fn new(git: Git) -> Self {
        Self {
            git,
            repos: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn set_repos(&self, repos: Vec<RepoFetchTarget>) {
        *self.repos.write().await = repos;
    }

    /// Fetches every repo and recounts its workspaces. Repos fetch side by side: each has its
    /// own git queue, and one slow remote shouldn't hold the others' counts back.
    pub async fn tick(&self) -> Vec<BehindUpdate> {
        let repos = self.repos.read().await.clone();
        let mut tasks = JoinSet::new();
        for repo in repos {
            let git = self.git.clone();
            tasks.spawn(async move { fetch_repo(&git, repo).await });
        }
        let mut updates = Vec::new();
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(repo_updates) => updates.extend(repo_updates),
                Err(error) => log::warn!("fetch task failed: {error}"),
            }
        }
        updates
    }

    pub fn spawn_loop(&self) {
        let scheduler = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(FETCH_INTERVAL);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                interval.tick().await;
                scheduler.tick().await;
            }
        });
    }
}

async fn fetch_repo(git: &Git, repo: RepoFetchTarget) -> Vec<BehindUpdate> {
    if let Err(error) = git.fetch(&repo.repo_path).await {
        log::warn!("fetch {} failed: {error}", repo.repo_path.display());
        return Vec::new();
    }
    let mut updates = Vec::with_capacity(repo.workspaces.len());
    for workspace in repo.workspaces {
        match git
            .behind_count(&workspace.worktree_path, &workspace.base)
            .await
        {
            Ok(behind) => updates.push(BehindUpdate {
                workspace_id: workspace.workspace_id,
                behind,
            }),
            Err(error) => log::warn!(
                "behind count {} failed: {error}",
                workspace.worktree_path.display()
            ),
        }
    }
    updates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::Git;
    use crate::shell_env::ShellEnv;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn tick_on_empty_repo_list_is_ok() {
        let mut env = ShellEnv::new();
        env.load_or_inherit().await.unwrap();
        let git = Git::new(Arc::new(RwLock::new(env)));
        let scheduler = FetchScheduler::new(git);
        let updates = scheduler.tick().await;
        assert!(updates.is_empty());
    }
}
