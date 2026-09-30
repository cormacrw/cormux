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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeDiff {
    pub workspace_id: String,
    /// Branch the diff is taken against; `None` means uncommitted changes vs `HEAD`.
    pub base: Option<String>,
    pub files: Vec<DiffFile>,
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
    diff_base: Arc<Mutex<HashMap<String, Option<String>>>>,
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
            diff_base: Arc::new(Mutex::new(HashMap::new())),
            tx,
            rx: Arc::new(Mutex::new(Some(rx))),
        }
    }

    /// Diff against `base` (merge-base..HEAD), or uncommitted changes when `None`.
    pub fn set_diff_base(&self, workspace_id: &str, base: Option<String>) {
        self.diff_base
            .lock()
            .unwrap()
            .insert(workspace_id.to_string(), base);
    }

    /// Set `base` unless a target was already chosen for this workspace.
    pub fn default_diff_base(&self, workspace_id: &str, base: String) {
        self.diff_base
            .lock()
            .unwrap()
            .entry(workspace_id.to_string())
            .or_insert(Some(base));
    }

    pub fn diff_base(&self, workspace_id: &str) -> Option<String> {
        self.diff_base
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
        let diff_base = self.diff_base.clone();
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
                                let base = diff_base.lock().unwrap().get(&id).cloned().flatten();
                                match compute_diff(&git, &id, &path, base.as_deref()).await {
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
        self.diff_base.lock().unwrap().remove(workspace_id);
    }

    pub fn latest(&self, workspace_id: &str) -> Option<WorktreeDiff> {
        self.latest.lock().unwrap().get(workspace_id).cloned()
    }

    pub fn request_refresh(&self, workspace_id: &str) {
        let _ = self.tx.send(workspace_id.to_string());
    }

    /// The diff against `base`, whatever the Changes panel is comparing against.
    pub async fn compute_against(
        &self,
        workspace_id: &str,
        path: &Path,
        base: &str,
    ) -> Result<WorktreeDiff> {
        compute_diff(&self.git, workspace_id, path, Some(base)).await
    }

    pub async fn compute(&self, workspace_id: &str, path: &Path) -> Result<WorktreeDiff> {
        let base = self.diff_base(workspace_id);
        let diff = compute_diff(&self.git, workspace_id, path, base.as_deref()).await?;
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
    pr_base: Option<&str>,
) -> Result<WorktreeDiff> {
    let stats = if let Some(base) = pr_base {
        git.numstat_against_base(path, base).await?
    } else {
        git.numstat(path).await?
    };
    let mut files = Vec::new();
    for (file_path, added, deleted) in stats {
        let text = if let Some(base) = pr_base {
            git.diff_file_against_base(path, base, &file_path)
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
        base: pr_base.map(str::to_string),
        files,
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
