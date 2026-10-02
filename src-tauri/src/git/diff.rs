use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ignore::gitignore::Gitignore;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::sync::mpsc;

use crate::error::{Error, Result};
use crate::git::Git;

const DEBOUNCE: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DiffHunk {
    pub header: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DiffFile {
    pub path: String,
    pub added: u32,
    pub deleted: u32,
    pub hunks: Vec<DiffHunk>,
}

/// Lines added and deleted across a set of files.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LineCounts {
    pub added: u32,
    pub deleted: u32,
}

impl LineCounts {
    fn of(stats: &[(String, u32, u32)]) -> Self {
        stats
            .iter()
            .fold(Self::default(), |counts, (_, added, deleted)| Self {
                added: counts.added + added,
                deleted: counts.deleted + deleted,
            })
    }
}

/// Committed changes on `head` since it left `base` (`base...head`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DiffTarget {
    /// A branch, or `HEAD` for whatever the worktree has checked out.
    pub head: String,
    pub base: String,
}

impl DiffTarget {
    pub fn head_against(base: impl Into<String>) -> Self {
        Self {
            head: "HEAD".into(),
            base: base.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeDiff {
    pub workspace_id: String,
    /// What the diff shows; `None` means uncommitted changes vs `HEAD`.
    pub target: Option<DiffTarget>,
    pub files: Vec<DiffFile>,
    /// Uncommitted changes vs `HEAD`, whatever `target` is.
    pub uncommitted: LineCounts,
}

struct Watched {
    workspace_id: String,
    path: PathBuf,
    _watcher: RecommendedWatcher,
}

/// `notify` watcher per worktree; debounced refresh filtered by `.gitignore`.
#[derive(Clone)]
pub struct LiveDiffEngine {
    git: Git,
    watched: Arc<Mutex<HashMap<String, Watched>>>,
    latest: Arc<Mutex<HashMap<String, WorktreeDiff>>>,
    /// Absent: no choice made yet. `Some(None)`: explicitly uncommitted.
    diff_target: Arc<Mutex<HashMap<String, Option<DiffTarget>>>>,
    tx: mpsc::UnboundedSender<String>,
    rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<String>>>>,
}

impl LiveDiffEngine {
    pub fn new(git: Git) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            git,
            watched: Arc::new(Mutex::new(HashMap::new())),
            latest: Arc::new(Mutex::new(HashMap::new())),
            diff_target: Arc::new(Mutex::new(HashMap::new())),
            tx,
            rx: Arc::new(Mutex::new(Some(rx))),
        }
    }

    /// Diff a branch against its base, or uncommitted changes when `None`.
    pub fn set_diff_target(&self, workspace_id: &str, target: Option<DiffTarget>) {
        self.diff_target
            .lock()
            .unwrap()
            .insert(workspace_id.to_string(), target);
    }

    /// Set `target` unless one was already chosen for this workspace.
    pub fn default_diff_target(&self, workspace_id: &str, target: DiffTarget) {
        self.diff_target
            .lock()
            .unwrap()
            .entry(workspace_id.to_string())
            .or_insert(Some(target));
    }

    pub fn diff_target(&self, workspace_id: &str) -> Option<DiffTarget> {
        self.diff_target
            .lock()
            .unwrap()
            .get(workspace_id)
            .cloned()
            .flatten()
    }

    /// Start the debounce loop. Must run after a Tokio/Tauri runtime exists.
    pub fn start(&self) {
        let Some(rx) = self.rx.lock().unwrap().take() else {
            return;
        };
        self.spawn_debouncer(rx);
    }

    fn spawn_debouncer(&self, mut rx: mpsc::UnboundedReceiver<String>) {
        let git = self.git.clone();
        let watched = self.watched.clone();
        let latest = self.latest.clone();
        let diff_target = self.diff_target.clone();
        tauri::async_runtime::spawn(async move {
            let mut pending: HashMap<String, tokio::time::Instant> = HashMap::new();
            let mut ticker = tokio::time::interval(Duration::from_millis(50));
            loop {
                tokio::select! {
                    Some(id) = rx.recv() => {
                        pending.insert(id, tokio::time::Instant::now());
                    }
                    _ = ticker.tick() => {
                        let ready: Vec<String> = pending
                            .iter()
                            .filter(|(_, instant)| instant.elapsed() >= DEBOUNCE)
                            .map(|(id, _)| id.clone())
                            .collect();
                        for id in ready {
                            pending.remove(&id);
                            let path = {
                                let guard = watched.lock().unwrap();
                                guard.get(&id).map(|item| item.path.clone())
                            };
                            if let Some(path) = path {
                                let target = diff_target.lock().unwrap().get(&id).cloned().flatten();
                                match compute_diff(&git, &id, &path, target.as_ref()).await {
                                    Ok(diff) => {
                                        latest.lock().unwrap().insert(id, diff);
                                    }
                                    Err(error) => log::warn!("diff refresh failed: {error}"),
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    pub fn watch(&self, workspace_id: &str, path: &Path) -> Result<()> {
        if self
            .watched
            .lock()
            .unwrap()
            .get(workspace_id)
            .is_some_and(|item| item.path == path)
        {
            return Ok(());
        }
        let tx = self.tx.clone();
        let id = workspace_id.to_string();
        let root = path.to_path_buf();
        let gitignore = gitignore_for(&root);
        let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
            let Ok(event) = result else {
                return;
            };
            if event
                .paths
                .iter()
                .any(|changed| should_refresh(&root, changed, &gitignore))
            {
                let _ = tx.send(id.clone());
            }
        })
        .map_err(|error| Error::Git(error.to_string()))?;
        watcher
            .watch(path, RecursiveMode::Recursive)
            .map_err(|error| Error::Git(error.to_string()))?;
        self.watched.lock().unwrap().insert(
            workspace_id.to_string(),
            Watched {
                workspace_id: workspace_id.to_string(),
                path: path.to_path_buf(),
                _watcher: watcher,
            },
        );
        let _ = self.tx.send(workspace_id.to_string());
        Ok(())
    }

    pub fn unwatch(&self, workspace_id: &str) {
        self.watched.lock().unwrap().remove(workspace_id);
        self.latest.lock().unwrap().remove(workspace_id);
        self.diff_target.lock().unwrap().remove(workspace_id);
    }

    pub fn latest(&self, workspace_id: &str) -> Option<WorktreeDiff> {
        self.latest.lock().unwrap().get(workspace_id).cloned()
    }

    pub fn request_refresh(&self, workspace_id: &str) {
        let _ = self.tx.send(workspace_id.to_string());
    }

    /// `HEAD` against `base`, whatever the Changes panel is showing.
    pub async fn compute_against(
        &self,
        workspace_id: &str,
        path: &Path,
        base: &str,
    ) -> Result<WorktreeDiff> {
        let target = DiffTarget::head_against(base);
        compute_diff(&self.git, workspace_id, path, Some(&target)).await
    }

    pub async fn compute(&self, workspace_id: &str, path: &Path) -> Result<WorktreeDiff> {
        let target = self.diff_target(workspace_id);
        let diff = compute_diff(&self.git, workspace_id, path, target.as_ref()).await?;
        self.latest
            .lock()
            .unwrap()
            .insert(workspace_id.to_string(), diff.clone());
        Ok(diff)
    }
}

fn gitignore_for(root: &Path) -> Gitignore {
    let mut builder = ignore::gitignore::GitignoreBuilder::new(root);
    let _ = builder.add(root.join(".gitignore"));
    builder.build().unwrap_or(Gitignore::empty())
}

fn should_refresh(root: &Path, path: &Path, gitignore: &Gitignore) -> bool {
    if path
        .components()
        .any(|component| component.as_os_str() == ".git")
    {
        return false;
    }
    let is_dir = path.is_dir();
    let relative = path.strip_prefix(root).unwrap_or(path);
    !gitignore.matched(relative, is_dir).is_ignore()
}

async fn compute_diff(
    git: &Git,
    workspace_id: &str,
    path: &Path,
    target: Option<&DiffTarget>,
) -> Result<WorktreeDiff> {
    let range = match target {
        Some(target) => Some(format!(
            "{}...{}",
            git.freshest_ref(path, &target.base).await,
            target.head
        )),
        None => None,
    };
    let (stats, uncommitted) = if let Some(range) = &range {
        let uncommitted = git.numstat(path).await.unwrap_or_default();
        (
            git.numstat_range(path, range).await?,
            LineCounts::of(&uncommitted),
        )
    } else {
        let stats = git.numstat(path).await?;
        let uncommitted = LineCounts::of(&stats);
        (stats, uncommitted)
    };
    let mut files = Vec::new();
    for (file_path, added, deleted) in stats {
        let text = if let Some(range) = &range {
            git.diff_file_range(path, range, &file_path)
                .await
                .unwrap_or_default()
        } else {
            git.diff_file(path, &file_path).await.unwrap_or_default()
        };
        files.push(DiffFile {
            path: file_path,
            added,
            deleted,
            hunks: parse_hunks(&text),
        });
    }
    Ok(WorktreeDiff {
        workspace_id: workspace_id.to_string(),
        target: target.cloned(),
        files,
        uncommitted,
    })
}

pub fn parse_hunks(diff: &str) -> Vec<DiffHunk> {
    let mut hunks = Vec::new();
    let mut current_header = String::new();
    let mut current_body = String::new();
    for line in diff.lines() {
        if line.starts_with("@@") {
            if !current_header.is_empty() {
                hunks.push(DiffHunk {
                    header: current_header,
                    body: current_body,
                });
                current_body = String::new();
            }
            current_header = line.to_string();
        } else if !current_header.is_empty() {
            current_body.push_str(line);
            current_body.push('\n');
        }
    }
    if !current_header.is_empty() {
        hunks.push(DiffHunk {
            header: current_header,
            body: current_body,
        });
    }
    hunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_unified_diff_into_hunks() {
        let diff = "diff --git a/a.txt b/a.txt\n--- a/a.txt\n+++ b/a.txt\n@@ -1,1 +1,2 @@\n hello\n+world\n";
        let hunks = parse_hunks(diff);
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].header, "@@ -1,1 +1,2 @@");
        assert!(hunks[0].body.contains("+world"));
    }

    fn commit_file(repo: &Path, name: &str) {
        std::fs::write(repo.join(name), "one\ntwo\n").unwrap();
        for args in [
            vec!["add", name],
            vec![
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-q",
                "-m",
                name,
            ],
        ] {
            let status = std::process::Command::new("git")
                .args(&args)
                .current_dir(repo)
                .status()
                .unwrap();
            assert!(status.success(), "git {args:?}");
        }
    }

    #[tokio::test]
    async fn a_branch_diffs_against_its_base_not_the_checkout() {
        use crate::shell_env::ShellEnv;
        use tokio::sync::RwLock;

        let git = Git::new(Arc::new(RwLock::new(ShellEnv::new())));
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(repo)
                .status()
                .unwrap();
            assert!(status.success(), "git {args:?}");
        };
        run(&["init", "-q", "-b", "main"]);
        commit_file(repo, "base.txt");
        run(&["switch", "-q", "-c", "feat/a"]);
        commit_file(repo, "a.txt");
        run(&["switch", "-q", "-c", "feat/b"]);
        commit_file(repo, "b.txt");
        std::fs::write(repo.join("wip.txt"), "wip\n").unwrap();

        let paths = |diff: WorktreeDiff| -> Vec<String> {
            diff.files.into_iter().map(|file| file.path).collect()
        };
        let target = |head: &str, base: &str| DiffTarget {
            head: head.into(),
            base: base.into(),
        };
        let lower = compute_diff(&git, "w", repo, Some(&target("feat/a", "main")))
            .await
            .unwrap();
        assert_eq!(paths(lower), ["a.txt"]);
        let upper = compute_diff(&git, "w", repo, Some(&target("feat/b", "feat/a")))
            .await
            .unwrap();
        assert_eq!(paths(upper), ["b.txt"]);
        let uncommitted = compute_diff(&git, "w", repo, None).await.unwrap();
        assert_eq!(paths(uncommitted), ["wip.txt"]);

        // feat/a needs a rebase once main moves on without it.
        assert!(git.is_ancestor(repo, "main", "feat/a").await);
        run(&["switch", "-q", "main"]);
        commit_file(repo, "later.txt");
        assert!(!git.is_ancestor(repo, "main", "feat/a").await);
        assert!(git.is_ancestor(repo, "feat/a", "feat/b").await);
    }

    #[test]
    fn ignores_git_dir_and_gitignored_files() {
        let gitignore = Gitignore::empty();
        assert!(!should_refresh(
            Path::new("/repo"),
            Path::new("/repo/.git/HEAD"),
            &gitignore
        ));
    }
}
