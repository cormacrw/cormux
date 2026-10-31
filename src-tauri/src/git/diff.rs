use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ignore::gitignore::Gitignore;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::sync::{Semaphore, mpsc, watch};
use tokio::time::{Instant, sleep_until};

use crate::error::{Error, Result};
use crate::git::{FileStat, Git};

const DEBOUNCE: Duration = Duration::from_millis(300);
/// Worktrees recomputing at once, however many are open.
const MAX_CONCURRENT_REFRESHES: usize = 4;
/// A file's patch beyond this stops at the last whole hunk that fits.
const MAX_FILE_PATCH_BYTES: usize = 1024 * 1024;
const GENERATED_DIRS: [&str; 7] = [
    "node_modules",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".turbo",
    "__pycache__",
    ".venv",
];
const CARGO_OUTPUT: [&str; 6] = [
    "debug",
    "release",
    "doc",
    "tmp",
    "rust-analyzer",
    "CACHEDIR.TAG",
];

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
    /// Where a renamed or moved file came from.
    pub old_path: Option<String>,
    pub added: u32,
    pub deleted: u32,
    pub hunks: Vec<DiffHunk>,
    /// Set when the patch was too big to send whole; `hunks` is the part that fit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub truncated: Option<bool>,
}

/// Lines added and deleted across a set of files.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LineCounts {
    pub added: u32,
    pub deleted: u32,
}

impl LineCounts {
    fn of<'a>(stats: impl IntoIterator<Item = &'a FileStat>) -> Self {
        stats
            .into_iter()
            .fold(Self::default(), |counts, stat| Self {
                added: counts.added + stat.added,
                deleted: counts.deleted + stat.deleted,
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
    path: PathBuf,
    _watcher: RecommendedWatcher,
}

/// Debounced diffs for the workspaces someone is looking at. Only a subscribed workspace has a
/// `notify` watcher or a stored diff, so background worktrees cost nothing here.
#[derive(Clone)]
pub struct LiveDiffEngine {
    git: Git,
    paths: Arc<Mutex<HashMap<String, PathBuf>>>,
    watched: Arc<Mutex<HashMap<String, Watched>>>,
    subscribers: Arc<Mutex<HashMap<String, usize>>>,
    latest: Arc<Mutex<HashMap<String, Arc<WorktreeDiff>>>>,
    /// Bumped whenever a stored diff actually changes.
    updates: Arc<watch::Sender<u64>>,
    /// Workspaces being recomputed, and whether another refresh came in meanwhile.
    in_flight: Arc<Mutex<HashMap<String, bool>>>,
    /// Absent: no choice made yet. `Some(None)`: explicitly uncommitted.
    diff_target: Arc<Mutex<HashMap<String, Option<DiffTarget>>>>,
    tx: mpsc::UnboundedSender<String>,
    rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<String>>>>,
}

/// Keeps a workspace's diff live until dropped.
pub struct DiffSubscription {
    engine: LiveDiffEngine,
    workspace_id: String,
}

impl Drop for DiffSubscription {
    fn drop(&mut self) {
        self.engine.unsubscribe(&self.workspace_id);
    }
}

impl LiveDiffEngine {
    pub fn new(git: Git) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            git,
            paths: Arc::new(Mutex::new(HashMap::new())),
            watched: Arc::new(Mutex::new(HashMap::new())),
            subscribers: Arc::new(Mutex::new(HashMap::new())),
            latest: Arc::new(Mutex::new(HashMap::new())),
            updates: Arc::new(watch::channel(0).0),
            in_flight: Arc::new(Mutex::new(HashMap::new())),
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
        let engine = self.clone();
        tauri::async_runtime::spawn(async move {
            let limit = Arc::new(Semaphore::new(MAX_CONCURRENT_REFRESHES));
            let mut pending: HashMap<String, Instant> = HashMap::new();
            loop {
                let next = pending.values().min().map(|at| *at + DEBOUNCE);
                tokio::select! {
                    received = rx.recv() => {
                        let Some(id) = received else { return };
                        pending.insert(id, Instant::now());
                    }
                    () = sleep_until(next.unwrap_or_else(Instant::now)), if next.is_some() => {
                        pending.retain(|id, at| {
                            if at.elapsed() < DEBOUNCE {
                                return true;
                            }
                            engine.refresh(id, &limit);
                            false
                        });
                    }
                }
            }
        });
    }

    /// Recompute off the debounce loop, one at a time per workspace, so a slow repo holds up
    /// neither the others nor the events behind it.
    fn refresh(&self, workspace_id: &str, limit: &Arc<Semaphore>) {
        if !self.is_subscribed(workspace_id) {
            return;
        }
        {
            let mut in_flight = self.in_flight.lock().unwrap();
            if let Some(again) = in_flight.get_mut(workspace_id) {
                *again = true;
                return;
            }
            in_flight.insert(workspace_id.to_string(), false);
        }
        let engine = self.clone();
        let limit = limit.clone();
        let id = workspace_id.to_string();
        tauri::async_runtime::spawn(async move {
            if let Ok(_permit) = limit.acquire().await {
                let path = engine.paths.lock().unwrap().get(&id).cloned();
                if let Some(path) = path
                    && let Err(error) = engine.compute(&id, &path).await
                {
                    log::warn!("diff refresh failed: {error}");
                }
            }
            let again = engine.in_flight.lock().unwrap().remove(&id);
            if again == Some(true) {
                let _ = engine.tx.send(id);
            }
        });
    }

    /// Remember where a workspace lives. Nothing is watched until someone subscribes.
    pub fn watch(&self, workspace_id: &str, path: &Path) -> Result<()> {
        self.paths
            .lock()
            .unwrap()
            .insert(workspace_id.to_string(), path.to_path_buf());
        if self.is_subscribed(workspace_id) {
            self.start_watching(workspace_id, path)?;
        }
        Ok(())
    }

    /// Keep this workspace's diff live until the returned handle is dropped.
    pub fn subscribe(&self, workspace_id: &str) -> DiffSubscription {
        *self
            .subscribers
            .lock()
            .unwrap()
            .entry(workspace_id.to_string())
            .or_insert(0) += 1;
        let path = self.paths.lock().unwrap().get(workspace_id).cloned();
        if let Some(path) = path
            && let Err(error) = self.start_watching(workspace_id, &path)
        {
            log::warn!("diff watch for {workspace_id} failed: {error}");
        }
        DiffSubscription {
            engine: self.clone(),
            workspace_id: workspace_id.to_string(),
        }
    }

    fn unsubscribe(&self, workspace_id: &str) {
        {
            let mut subscribers = self.subscribers.lock().unwrap();
            let Some(count) = subscribers.get_mut(workspace_id) else {
                return;
            };
            *count -= 1;
            if *count > 0 {
                return;
            }
            subscribers.remove(workspace_id);
        }
        self.watched.lock().unwrap().remove(workspace_id);
        self.latest.lock().unwrap().remove(workspace_id);
    }

    fn is_subscribed(&self, workspace_id: &str) -> bool {
        self.subscribers.lock().unwrap().contains_key(workspace_id)
    }

    fn start_watching(&self, workspace_id: &str, path: &Path) -> Result<()> {
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
        let canonical_root = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let gitignore = gitignore_for(&root);
        // A commit only touches git's own files, which may live outside the
        // worktree (a linked worktree's are under the main repo's `.git`).
        let git_dirs = git_dirs(&root);
        let state_dirs = git_dirs.clone();
        let event_root = canonical_root.clone();
        let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
            let Ok(event) = result else {
                return;
            };
            if event.paths.iter().any(|changed| {
                should_refresh(&root, changed, &gitignore)
                    || should_refresh(&event_root, changed, &gitignore)
                    || moves_git_state(&state_dirs, changed)
            }) {
                let _ = tx.send(id.clone());
            }
        })
        .map_err(|error| Error::Git(error.to_string()))?;
        watcher
            .watch(path, RecursiveMode::Recursive)
            .map_err(|error| Error::Git(error.to_string()))?;
        for dir in git_dirs
            .iter()
            .filter(|dir| !dir.starts_with(&canonical_root))
        {
            if let Err(error) = watcher.watch(dir, RecursiveMode::Recursive) {
                log::warn!("watching {} failed: {error}", dir.display());
            }
        }
        self.watched.lock().unwrap().insert(
            workspace_id.to_string(),
            Watched {
                path: path.to_path_buf(),
                _watcher: watcher,
            },
        );
        let _ = self.tx.send(workspace_id.to_string());
        Ok(())
    }

    pub fn unwatch(&self, workspace_id: &str) {
        self.paths.lock().unwrap().remove(workspace_id);
        self.watched.lock().unwrap().remove(workspace_id);
        self.latest.lock().unwrap().remove(workspace_id);
        self.diff_target.lock().unwrap().remove(workspace_id);
    }

    pub fn latest(&self, workspace_id: &str) -> Option<Arc<WorktreeDiff>> {
        self.latest.lock().unwrap().get(workspace_id).cloned()
    }

    /// Changes whenever any workspace's stored diff does.
    pub fn updates(&self) -> watch::Receiver<u64> {
        self.updates.subscribe()
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
        self.publish(workspace_id, &diff);
        Ok(diff)
    }

    /// Store a diff for subscribers, waking them only when it differs from the last one.
    fn publish(&self, workspace_id: &str, diff: &WorktreeDiff) {
        if !self.is_subscribed(workspace_id) {
            return;
        }
        let mut latest = self.latest.lock().unwrap();
        if latest.get(workspace_id).is_some_and(|last| **last == *diff) {
            return;
        }
        latest.insert(workspace_id.to_string(), Arc::new(diff.clone()));
        drop(latest);
        self.updates.send_modify(|version| *version += 1);
    }
}

fn gitignore_for(root: &Path) -> Gitignore {
    let mut builder = ignore::gitignore::GitignoreBuilder::new(root);
    let _ = builder.add(root.join(".gitignore"));
    builder.build().unwrap_or(Gitignore::empty())
}

/// Dependency and build output: churns constantly during installs and builds, and is ignored by
/// a `.gitignore` that may sit below the worktree root, where `gitignore_for` doesn't look.
fn is_generated_path(relative: &Path) -> bool {
    let mut parts = relative.components().map(|part| part.as_os_str());
    while let Some(part) = parts.next() {
        if part == "target" {
            // A folder merely named `target` can hold real source; cargo's has these inside.
            if parts
                .clone()
                .next()
                .is_some_and(|next| CARGO_OUTPUT.iter().any(|name| next == *name))
            {
                return true;
            }
        } else if GENERATED_DIRS.iter().any(|name| part == *name) {
            return true;
        }
    }
    false
}

fn should_refresh(root: &Path, path: &Path, gitignore: &Gitignore) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    if relative.as_os_str().is_empty() {
        return true;
    }
    if relative
        .components()
        .any(|component| component.as_os_str() == ".git")
        || is_generated_path(relative)
    {
        return false;
    }
    // Directories are checked as parents of their contents; a change to the directory itself
    // refreshes at worst once, which beats a stat on every event.
    !gitignore
        .matched_path_or_any_parents(relative, false)
        .is_ignore()
}

/// The worktree's git dir and, for a linked worktree, the shared one.
fn git_dirs(root: &Path) -> Vec<PathBuf> {
    let Some(git_dir) = crate::git::worktree_git_dir(root) else {
        return Vec::new();
    };
    let git_dir = git_dir.canonicalize().unwrap_or(git_dir);
    let mut dirs = vec![git_dir.clone()];
    if let Ok(common) = std::fs::read_to_string(git_dir.join("commondir")) {
        let common = git_dir.join(common.trim());
        dirs.push(common.canonicalize().unwrap_or(common));
    }
    dirs
}

/// Whether `path`, reported by the watcher, moved this worktree's own git state. The first dir
/// is the worktree's; the shared one's `HEAD` and index belong to the main checkout.
fn moves_git_state(git_dirs: &[PathBuf], path: &Path) -> bool {
    let mut dirs = git_dirs.iter();
    dirs.next().is_some_and(|own| is_git_state(own, path))
        || dirs.any(|shared| is_shared_ref(shared, path))
}

/// Files that move `HEAD` or the index: commits, resets, checkouts, staging.
fn is_git_state(git_dir: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(git_dir) else {
        return false;
    };
    relative == Path::new("HEAD") || relative == Path::new("index") || is_ref(relative)
}

fn is_shared_ref(git_dir: &Path, path: &Path) -> bool {
    path.strip_prefix(git_dir).is_ok_and(is_ref)
}

fn is_ref(relative: &Path) -> bool {
    relative == Path::new("packed-refs")
        || relative.starts_with("refs/heads")
        || relative.starts_with("refs/remotes")
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
    let (changes, uncommitted) = if let Some(range) = &range {
        let uncommitted = git.numstat(path).await.unwrap_or_default();
        let changes = git.range_changes(path, range).await?;
        (changes, LineCounts::of(&uncommitted))
    } else {
        let changes = git.uncommitted_changes(path, true).await?;
        let uncommitted = LineCounts::of(changes.iter().map(|(stat, _)| stat));
        (changes, uncommitted)
    };
    let files = changes
        .into_iter()
        .map(|(stat, patch)| {
            let (hunks, truncated) = capped_hunks(parse_hunks(&patch));
            DiffFile {
                path: stat.path,
                old_path: stat.old_path,
                added: stat.added,
                deleted: stat.deleted,
                hunks,
                truncated: truncated.then_some(true),
            }
        })
        .collect();
    Ok(WorktreeDiff {
        workspace_id: workspace_id.to_string(),
        target: target.cloned(),
        files,
        uncommitted,
    })
}

/// The whole hunks that fit in `MAX_FILE_PATCH_BYTES`, and whether any didn't.
fn capped_hunks(mut hunks: Vec<DiffHunk>) -> (Vec<DiffHunk>, bool) {
    let mut bytes = 0;
    let fits = hunks
        .iter()
        .take_while(|hunk| {
            bytes += hunk.header.len() + hunk.body.len();
            bytes <= MAX_FILE_PATCH_BYTES
        })
        .count();
    let truncated = fits < hunks.len();
    hunks.truncate(fits);
    (hunks, truncated)
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
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join(".gitignore"), "dist/\n*.log\n").unwrap();
        let gitignore = gitignore_for(root);
        let refreshes = |path: &str| should_refresh(root, &root.join(path), &gitignore);
        assert!(!refreshes(".git/HEAD"));
        assert!(!refreshes("dist/deep/bundle.js"));
        assert!(!refreshes("debug.log"));
        assert!(!refreshes("node_modules/left-pad/index.js"));
        assert!(!refreshes("packages/app/node_modules/x/y.js"));
        assert!(!refreshes("src-tauri/target/debug/cormux"));
        assert!(refreshes("src/target/main.rs"));
        assert!(refreshes("src/main.rs"));
        // A path outside the root must not reach the matcher, which panics on one.
        assert!(!should_refresh(
            root,
            Path::new("/elsewhere/file.txt"),
            &gitignore
        ));
    }

    #[test]
    fn caps_a_files_patch_at_a_hunk_boundary() {
        let hunk = |bytes: usize| DiffHunk {
            header: "@@ -1 +1 @@".into(),
            body: "x".repeat(bytes),
        };
        let (kept, truncated) = capped_hunks(vec![hunk(10), hunk(20)]);
        assert_eq!((kept.len(), truncated), (2, false));
        let (kept, truncated) = capped_hunks(vec![hunk(10), hunk(MAX_FILE_PATCH_BYTES), hunk(10)]);
        assert_eq!((kept.len(), truncated), (1, true));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn only_subscribed_workspaces_are_diffed() {
        use crate::shell_env::ShellEnv;
        use tokio::sync::RwLock;

        let engine = LiveDiffEngine::new(Git::new(Arc::new(RwLock::new(ShellEnv::new()))));
        engine.start();
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        assert!(
            std::process::Command::new("git")
                .args(["init", "-q", "-b", "main"])
                .current_dir(repo)
                .status()
                .unwrap()
                .success()
        );
        commit_file(repo, "a.txt");
        engine.watch("ws", repo).unwrap();
        engine.compute("ws", repo).await.unwrap();
        assert!(engine.latest("ws").is_none());
        assert!(engine.watched.lock().unwrap().is_empty());

        let first = engine.subscribe("ws");
        let second = engine.subscribe("ws");
        let mut updates = engine.updates();
        tokio::time::timeout(Duration::from_secs(5), updates.changed())
            .await
            .unwrap()
            .unwrap();
        assert!(engine.latest("ws").is_some());
        assert_eq!(engine.watched.lock().unwrap().len(), 1);

        drop(first);
        assert!(engine.latest("ws").is_some());
        drop(second);
        assert!(engine.latest("ws").is_none());
        assert!(engine.watched.lock().unwrap().is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_commit_in_a_linked_worktree_clears_the_counts() {
        use crate::shell_env::ShellEnv;
        use tokio::sync::RwLock;

        let engine = LiveDiffEngine::new(Git::new(Arc::new(RwLock::new(ShellEnv::new()))));
        engine.start();
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        let tree = dir.path().join("tree");
        std::fs::create_dir(&repo).unwrap();
        let git = |cwd: &Path, args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(["-c", "user.name=t", "-c", "user.email=t@t"])
                .args(args)
                .current_dir(cwd)
                .status()
                .unwrap();
            assert!(status.success(), "git {args:?}");
        };
        git(&repo, &["init", "-q", "-b", "main"]);
        commit_file(&repo, "a.txt");
        git(&repo, &["worktree", "add", "-q", tree.to_str().unwrap()]);
        engine.watch("ws", &tree).unwrap();
        let _subscription = engine.subscribe("ws");

        let added =
            |engine: &LiveDiffEngine| engine.latest("ws").map(|diff| diff.uncommitted.added);
        let wait_for = async |want: u32| {
            for _ in 0..50 {
                if added(&engine) == Some(want) {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            panic!(
                "uncommitted added never became {want}: {:?}",
                added(&engine)
            );
        };
        wait_for(0).await;
        std::fs::write(tree.join("b.txt"), "one\ntwo\n").unwrap();
        wait_for(2).await;
        git(&tree, &["add", "b.txt"]);
        git(&tree, &["commit", "-q", "-m", "b"]);
        wait_for(0).await;
    }

    #[test]
    fn refreshes_on_head_index_and_ref_changes() {
        let git_dir = Path::new("/repo/.git");
        assert!(is_git_state(git_dir, Path::new("/repo/.git/HEAD")));
        assert!(is_git_state(git_dir, Path::new("/repo/.git/index")));
        assert!(is_git_state(git_dir, Path::new("/repo/.git/refs/heads/a")));
        assert!(!is_git_state(git_dir, Path::new("/repo/.git/refs/tags/v1")));
        let dirs = [PathBuf::from("/repo/.git/worktrees/a"), git_dir.into()];
        assert!(moves_git_state(
            &dirs,
            Path::new("/repo/.git/worktrees/a/index")
        ));
        assert!(moves_git_state(&dirs, Path::new("/repo/.git/refs/heads/a")));
        // The main checkout staging something changes nothing here.
        assert!(!moves_git_state(&dirs, Path::new("/repo/.git/index")));
        assert!(!is_git_state(
            git_dir,
            Path::new("/repo/.git/objects/ab/cd")
        ));
        assert!(!is_git_state(git_dir, Path::new("/repo/src/main.rs")));
    }

    #[test]
    fn finds_a_linked_worktrees_git_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        let tree = dir.path().join("tree");
        std::fs::create_dir(&repo).unwrap();
        let run = |cwd: &Path, args: &[&str]| {
            assert!(
                std::process::Command::new("git")
                    .args(args)
                    .current_dir(cwd)
                    .status()
                    .unwrap()
                    .success()
            );
        };
        run(&repo, &["init", "-q", "-b", "main"]);
        commit_file(&repo, "a.txt");
        run(&repo, &["worktree", "add", "-q", tree.to_str().unwrap()]);
        let common = repo.join(".git").canonicalize().unwrap();
        assert_eq!(
            git_dirs(&tree),
            vec![common.join("worktrees/tree"), common.clone()]
        );
        assert_eq!(git_dirs(&repo), vec![common]);
    }
}
