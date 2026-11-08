use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::task::JoinSet;

use crate::git::Git;

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
}

impl FetchScheduler {
    pub fn new(git: Git) -> Self {
        Self { git }
    }

    /// Fetches every repo and recounts its workspaces. Repos fetch side by side: each has its
    /// own git queue, and one slow remote shouldn't hold the others' counts back.
    pub async fn tick(&self, repos: Vec<RepoFetchTarget>) -> Vec<BehindUpdate> {
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

    use std::path::Path;

    fn git(cwd: &Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(cwd)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }

    fn commit(repo: &Path, file: &str) {
        std::fs::write(repo.join(file), "x\n").unwrap();
        git(repo, &["add", file]);
        git(
            repo,
            &[
                "-c",
                "user.name=C",
                "-c",
                "user.email=c@t",
                "commit",
                "-qm",
                file,
            ],
        );
    }

    #[tokio::test]
    async fn a_tick_fetches_and_recounts_and_skips_a_repo_it_cannot_fetch() {
        let dir = tempfile::tempdir().unwrap();
        let origin = dir.path().join("origin");
        std::fs::create_dir(&origin).unwrap();
        git(&origin, &["init", "-qb", "main"]);
        commit(&origin, "one.txt");
        git(dir.path(), &["clone", "-q", "origin", "clone"]);
        let clone = dir.path().join("clone");
        commit(&origin, "landed.txt");
        let broken = dir.path().join("broken");
        git(dir.path(), &["clone", "-q", "origin", "broken"]);
        git(&broken, &["remote", "set-url", "origin", "/nowhere"]);

        let target = |path: &Path, id: &str| RepoFetchTarget {
            repo_path: path.to_path_buf(),
            workspaces: vec![WorkspaceFetchTarget {
                workspace_id: id.into(),
                worktree_path: path.to_path_buf(),
                base: "main".into(),
            }],
        };
        let scheduler = FetchScheduler::new(Git::new(crate::shell_env::SharedEnv::loaded(
            crate::shell_env::ShellEnv::new(),
        )));
        let updates = scheduler
            .tick(vec![target(&clone, "w1"), target(&broken, "w2")])
            .await;
        let counts: Vec<_> = updates
            .iter()
            .map(|update| (update.workspace_id.as_str(), update.behind))
            .collect();
        assert_eq!(counts, [("w1", 1)]);
    }
}
