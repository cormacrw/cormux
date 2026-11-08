mod diff;
mod fetch;
mod queue;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::process::Command;
use tokio::sync::{Mutex as AsyncMutex, RwLock};

use crate::error::{Error, Result};
use crate::model::FALLBACK_DEFAULT_BRANCH;
use crate::shell_env::ShellEnv;

pub use diff::{DiffTarget, LiveDiffEngine, WorktreeDiff};
pub use fetch::{BehindUpdate, FetchScheduler, RepoFetchTarget, WorkspaceFetchTarget};
pub use queue::WorktreeQueue;

/// A merge stopped on these paths, waiting for the user to resolve or abort it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GitConflictState {
    pub paths: Vec<String>,
}

/// A new thread waits this long at most for its base branch to fetch.
const BASE_FETCH_TIMEOUT: Duration = Duration::from_secs(20);
const BACKGROUND_FETCH_TIMEOUT: Duration = Duration::from_secs(120);
/// New threads off the same base within this long share one fetch.
const BASE_FETCH_TTL: Duration = Duration::from_secs(30);

/// `git diff` with a stable header, whatever the user's diff settings, for `split_patches`.
const PATCH_ARGS: [&str; 8] = [
    "-c",
    "core.quotePath=false",
    "diff",
    "--no-ext-diff",
    "--no-color",
    "--src-prefix=a/",
    "--dst-prefix=b/",
    "-M",
];

/// System `git` wrapper. Credential helpers, hooks and LFS apply because this
/// shells out instead of using git2/gix. Calls for one worktree are serialised.
#[derive(Clone)]
pub struct Git {
    inner: Arc<GitInner>,
}

struct GitInner {
    env: Arc<RwLock<ShellEnv>>,
    queue: WorktreeQueue,
    /// When each `repo\0base` last fetched, locked for the duration of a fetch.
    base_fetches: Mutex<HashMap<String, Arc<AsyncMutex<Option<Instant>>>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalBranchTip {
    pub name: String,
    pub committed: String,
    pub subject: String,
}

/// Rejects a branch or base name git wouldn't accept as a ref, and any that git would read
/// as an option (`--upload-pack=…`) where it's passed positionally. Names reach git from the
/// webview and from agents (the `create_thread` tool's `base`), so they're checked here
/// rather than trusted. Follows `git check-ref-format`'s rules.
///
/// # Errors
///
/// `Error::Git` naming `what` and the rejected value.
pub fn validate_ref_name(name: &str, what: &str) -> Result<()> {
    let bad = name.is_empty()
        || name.starts_with('-')
        || name.starts_with('/')
        || name.ends_with('/')
        || name.ends_with('.')
        || name.ends_with(".lock")
        || name == "@"
        || name.contains("..")
        || name.contains("//")
        || name.contains("@{")
        || name.split('/').any(|part| part.starts_with('.'))
        || name
            .chars()
            .any(|c| c.is_ascii_control() || c.is_whitespace() || "~^:?*[\\".contains(c));
    if bad {
        return Err(Error::Git(format!("invalid {what} name: {name:?}")));
    }
    Ok(())
}

pub fn parse_local_branch_tips(stdout: &str) -> Vec<LocalBranchTip> {
    stdout
        .lines()
        .filter_map(|line| {
            if line.is_empty() {
                return None;
            }
            let mut parts = line.splitn(3, '\0');
            let name = parts.next()?.trim();
            if name.is_empty() {
                return None;
            }
            Some(LocalBranchTip {
                name: name.to_string(),
                committed: parts.next().unwrap_or("").to_string(),
                subject: parts.next().unwrap_or("").to_string(),
            })
        })
        .collect()
}

impl Git {
    pub fn new(env: Arc<RwLock<ShellEnv>>) -> Self {
        Self {
            inner: Arc::new(GitInner {
                env,
                queue: WorktreeQueue::default(),
                base_fetches: Mutex::new(HashMap::new()),
            }),
        }
    }

    pub fn queue(&self) -> &WorktreeQueue {
        &self.inner.queue
    }

    async fn with_queue<T, F, Fut>(&self, worktree: &Path, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        self.inner.queue.run(worktree, f).await
    }

    async fn run(&self, cwd: &Path, args: &[&str]) -> Result<Output> {
        self.with_queue(cwd, || async { self.run_unlocked(Some(cwd), args).await })
            .await
    }

    async fn run_unlocked(&self, cwd: Option<&Path>, args: &[&str]) -> Result<Output> {
        self.run_with_env(cwd, args, &[]).await
    }

    async fn run_with_env(
        &self,
        cwd: Option<&Path>,
        args: &[&str],
        extra_env: &[(&str, &str)],
    ) -> Result<Output> {
        let env = self.inner.env.read().await;
        let mut command = Command::new("git");
        command.args(args);
        env.apply(&mut command);
        command.envs(extra_env.iter().copied());
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        command.stdin(std::process::Stdio::null());
        // No terminal to answer a credential prompt; fail instead of hanging. And a caller
        // that times out (e.g. `fetch_base`) shouldn't leave git running behind it.
        command.env("GIT_TERMINAL_PROMPT", "0");
        command.kill_on_drop(true);
        let output = command
            .output()
            .await
            .map_err(|error| Error::Git(error.to_string()))?;
        Ok(output)
    }

    fn stdout(output: &Output) -> String {
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    fn fail(output: &Output, context: &str) -> Error {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Error::Git(format!("{context}: {stderr}").trim().to_string())
    }

    fn require_success(output: &Output, context: &str) -> Result<()> {
        if output.status.success() {
            Ok(())
        } else {
            Err(Self::fail(output, context))
        }
    }

    fn diff_ok(output: &Output, context: &str) -> Result<()> {
        match output.status.code() {
            Some(0 | 1) => Ok(()),
            _ => Err(Self::fail(output, context)),
        }
    }

    pub async fn show_toplevel(&self, path: &Path) -> Result<PathBuf> {
        let output = self.run(path, &["rev-parse", "--show-toplevel"]).await?;
        Self::require_success(&output, "rev-parse --show-toplevel")?;
        Ok(PathBuf::from(Self::stdout(&output).trim()))
    }

    pub async fn remote_origin_url(&self, repo: &Path) -> Result<Option<String>> {
        let output = self.run(repo, &["remote", "get-url", "origin"]).await?;
        if !output.status.success() {
            return Ok(None);
        }
        let url = Self::stdout(&output).trim().to_string();
        if url.is_empty() {
            Ok(None)
        } else {
            Ok(Some(url))
        }
    }

    pub async fn default_branch(&self, repo: &Path) -> Result<String> {
        let output = self
            .run(repo, &["symbolic-ref", "refs/remotes/origin/HEAD"])
            .await?;
        if output.status.success() {
            let value = Self::stdout(&output).trim().to_string();
            let branch = value
                .strip_prefix("refs/remotes/origin/")
                .unwrap_or(FALLBACK_DEFAULT_BRANCH)
                .to_string();
            return Ok(branch);
        }
        Ok(FALLBACK_DEFAULT_BRANCH.into())
    }

    pub async fn worktree_add(
        &self,
        repo: &Path,
        path: &Path,
        branch: &str,
        base: &str,
    ) -> Result<()> {
        let path_str = path.to_string_lossy().to_string();
        let output = self
            .run(
                repo,
                &[
                    "worktree",
                    "add",
                    "--no-track",
                    "-b",
                    branch,
                    &path_str,
                    base,
                ],
            )
            .await?;
        Self::require_success(&output, "worktree add")
    }

    /// Where a new branch off `base` should start: the freshest of local and origin's, fetched
    /// now. A fetch that fails (offline, no remote) falls back to what's already here.
    pub async fn latest_base(&self, repo: &Path, base: &str) -> String {
        self.fetch_base(repo, base).await;
        self.freshest_ref(repo, base).await
    }

    /// Fetch `base` unless that just happened. Threads started together wait on one fetch,
    /// outside the repo's queue, so nothing else on the repo waits for the network.
    async fn fetch_base(&self, repo: &Path, base: &str) {
        let key = format!("{}\0{base}", WorktreeQueue::key(repo));
        let slot = {
            let Ok(mut fetches) = self.inner.base_fetches.lock() else {
                return;
            };
            fetches.entry(key).or_default().clone()
        };
        let mut fetched = slot.lock().await;
        if fetched.is_some_and(|at| at.elapsed() < BASE_FETCH_TTL) {
            return;
        }
        // `--` so a `base` that starts with `-` is a refspec, not an option.
        let args = ["fetch", "origin", "--", base];
        let fetch = self.run_unlocked(Some(repo), &args);
        match tokio::time::timeout(BASE_FETCH_TIMEOUT, fetch).await {
            Ok(Ok(output)) if output.status.success() => *fetched = Some(Instant::now()),
            Ok(Ok(output)) => log::warn!(
                "fetching {base} failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
            Ok(Err(error)) => log::warn!("fetching {base} failed: {error}"),
            Err(_) => log::warn!("fetching {base} timed out"),
        }
    }

    pub async fn worktree_add_detached(
        &self,
        repo: &Path,
        path: &Path,
        start_point: &str,
    ) -> Result<()> {
        let path_str = path.to_string_lossy().to_string();
        let output = self
            .run(repo, &["worktree", "add", &path_str, start_point])
            .await?;
        Self::require_success(&output, "worktree add")
    }

    pub async fn fetch_pull_ref(&self, repo: &Path, number: u64, local_branch: &str) -> Result<()> {
        let refspec = format!("pull/{number}/head:{local_branch}");
        let output = self.run(repo, &["fetch", "origin", &refspec]).await?;
        Self::require_success(&output, "fetch pull ref")
    }

    /// Outside the repo's queue: deleting a worktree can take many seconds, and new threads
    /// need that queue to add theirs. Git is fine with adds and removes overlapping.
    pub async fn worktree_remove(&self, repo: &Path, path: &Path, force: bool) -> Result<()> {
        let path_str = path.to_string_lossy().to_string();
        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        args.push(&path_str);
        let output = self.run_unlocked(Some(repo), &args).await?;
        Self::require_success(&output, "worktree remove")
    }

    pub async fn behind_count(&self, worktree: &Path, base: &str) -> Result<u32> {
        let base = self.freshest_ref(worktree, base).await;
        self.rev_list_count(worktree, &format!("HEAD..{base}"))
            .await
    }

    /// Outside the repo's queue, like `fetch_base`: a slow network shouldn't stop new threads
    /// adding their worktrees, and a stuck fetch gives up rather than holding on forever.
    pub async fn fetch(&self, repo: &Path) -> Result<()> {
        let fetch = self.run_unlocked(Some(repo), &["fetch", "--all", "--prune"]);
        let output = tokio::time::timeout(BACKGROUND_FETCH_TIMEOUT, fetch)
            .await
            .map_err(|_| Error::Git("fetch timed out".into()))??;
        Self::require_success(&output, "fetch")
    }

    async fn resolve_ref(&self, repo: &Path, reference: &str) -> Result<Option<String>> {
        let output = self
            .run(repo, &["rev-parse", "--verify", "--quiet", reference])
            .await?;
        Ok(output
            .status
            .success()
            .then(|| Self::stdout(&output).trim().to_string()))
    }

    /// Uncommitted changes vs `HEAD`, untracked files included and renames paired.
    pub async fn numstat(&self, worktree: &Path) -> Result<Vec<FileStat>> {
        Ok(self
            .uncommitted_changes(worktree, false)
            .await?
            .into_iter()
            .map(|(stat, _)| stat)
            .collect())
    }

    /// Files changed by `range`, e.g. `main...feature`.
    pub async fn numstat_range(&self, worktree: &Path, range: &str) -> Result<Vec<FileStat>> {
        let output = self
            .run(worktree, &["diff", "--numstat", "-z", "-M", range])
            .await?;
        Self::diff_ok(&output, "diff --numstat")?;
        Ok(parse_numstat_z(&output.stdout))
    }

    /// `git diff HEAD` can't see untracked files, so a moved file would show as a
    /// deletion and a separate addition. They are marked intent-to-add in a copy of
    /// the index, which leaves the real one alone and lets git pair renames.
    pub async fn uncommitted_changes(
        &self,
        worktree: &Path,
        with_patches: bool,
    ) -> Result<Vec<(FileStat, String)>> {
        self.with_queue(worktree, || async {
            let index = self.scratch_index(worktree).await?;
            let result = self
                .uncommitted_changes_in(worktree, &index, with_patches)
                .await;
            let _ = tokio::fs::remove_file(&index).await;
            result
        })
        .await
    }

    async fn uncommitted_changes_in(
        &self,
        worktree: &Path,
        index: &Path,
        with_patches: bool,
    ) -> Result<Vec<(FileStat, String)>> {
        let index = index.to_string_lossy().to_string();
        let env = [("GIT_INDEX_FILE", index.as_str())];
        let added = self
            .run_with_env(Some(worktree), &["add", "--intent-to-add", "--", "."], &env)
            .await?;
        Self::require_success(&added, "add --intent-to-add")?;
        let output = self
            .run_with_env(
                Some(worktree),
                &["diff", "--numstat", "-z", "-M", "HEAD"],
                &env,
            )
            .await?;
        Self::diff_ok(&output, "diff --numstat")?;
        let stats = parse_numstat_z(&output.stdout);
        if !with_patches {
            return Ok(stats
                .into_iter()
                .map(|stat| (stat, String::new()))
                .collect());
        }
        let mut args = PATCH_ARGS.to_vec();
        args.push("HEAD");
        let whole = self.run_with_env(Some(worktree), &args, &env).await?;
        Self::diff_ok(&whole, "diff")?;
        with_patches_from(stats, &Self::stdout(&whole), |paths| async move {
            let mut args = vec!["--literal-pathspecs"];
            args.extend(PATCH_ARGS);
            args.extend(["HEAD", "--"]);
            args.extend(paths.iter().map(String::as_str));
            let output = self.run_with_env(Some(worktree), &args, &env).await?;
            Self::diff_ok(&output, "diff file")?;
            Ok(Self::stdout(&output))
        })
        .await
    }

    /// Throws away uncommitted changes to `paths`, or every uncommitted change when
    /// `paths` is empty: tracked files go back to `HEAD`, index included, and
    /// untracked files are deleted. Ignored files are left alone.
    pub async fn discard(&self, worktree: &Path, paths: &[String]) -> Result<()> {
        self.with_queue(worktree, || async {
            if paths.is_empty() {
                let restored = self
                    .run_unlocked(
                        Some(worktree),
                        &[
                            "restore",
                            "--source=HEAD",
                            "--staged",
                            "--worktree",
                            "--",
                            ":/",
                        ],
                    )
                    .await?;
                Self::require_success(&restored, "restore")?;
                let cleaned = self
                    .run_unlocked(Some(worktree), &["clean", "-f", "-d", "--", ":/"])
                    .await?;
                return Self::require_success(&cleaned, "clean");
            }
            // Paths are file names, not patterns: `app/[id]/page.tsx` must not also match
            // `app/i/page.tsx`.
            let mut args = vec![
                "--literal-pathspecs",
                "ls-files",
                "-z",
                "--others",
                "--exclude-standard",
                "--",
            ];
            args.extend(paths.iter().map(String::as_str));
            let listed = self.run_unlocked(Some(worktree), &args).await?;
            Self::require_success(&listed, "ls-files")?;
            let untracked: Vec<String> = Self::stdout(&listed)
                .split('\0')
                .filter(|path| !path.is_empty())
                .map(str::to_string)
                .collect();
            let tracked: Vec<&str> = paths
                .iter()
                .filter(|path| !untracked.contains(path))
                .map(String::as_str)
                .collect();
            if !tracked.is_empty() {
                let mut args = vec![
                    "--literal-pathspecs",
                    "restore",
                    "--source=HEAD",
                    "--staged",
                    "--worktree",
                    "--",
                ];
                args.extend(tracked);
                let restored = self.run_unlocked(Some(worktree), &args).await?;
                Self::require_success(&restored, "restore")?;
            }
            if !untracked.is_empty() {
                let mut args = vec!["--literal-pathspecs", "clean", "-f", "--"];
                args.extend(untracked.iter().map(String::as_str));
                let cleaned = self.run_unlocked(Some(worktree), &args).await?;
                Self::require_success(&cleaned, "clean")?;
            }
            Ok(())
        })
        .await
    }

    async fn scratch_index(&self, worktree: &Path) -> Result<PathBuf> {
        let output = self
            .run_unlocked(Some(worktree), &["rev-parse", "--git-path", "index"])
            .await?;
        Self::require_success(&output, "rev-parse --git-path index")?;
        let index = worktree.join(Self::stdout(&output).trim());
        let scratch = std::env::temp_dir().join(format!("cormux-index-{}", uuid::Uuid::new_v4()));
        tokio::fs::copy(&index, &scratch)
            .await
            .map_err(|error| Error::Git(error.to_string()))?;
        Ok(scratch)
    }

    pub async fn range_changes(
        &self,
        worktree: &Path,
        range: &str,
    ) -> Result<Vec<(FileStat, String)>> {
        let stats = self.numstat_range(worktree, range).await?;
        if stats.is_empty() {
            return Ok(Vec::new());
        }
        let mut args = PATCH_ARGS.to_vec();
        args.push(range);
        let whole = self.run(worktree, &args).await?;
        Self::diff_ok(&whole, "diff range")?;
        with_patches_from(stats, &Self::stdout(&whole), |paths| async move {
            let mut args = vec!["--literal-pathspecs"];
            args.extend(PATCH_ARGS);
            args.extend([range, "--"]);
            args.extend(paths.iter().map(String::as_str));
            let output = self.run(worktree, &args).await?;
            // The file still lists, with its counts, when only its patch can't be shown.
            if let Err(error) = Self::diff_ok(&output, "diff file in range") {
                log::warn!("{error}");
                return Ok(String::new());
            }
            Ok(Self::stdout(&output))
        })
        .await
    }

    pub async fn status_porcelain(&self, worktree: &Path) -> Result<String> {
        // A read: don't take `index.lock` to refresh the index, which would stall behind
        // (or trip up) an agent running its own git commands in the worktree.
        let output = self
            .run(worktree, &["--no-optional-locks", "status", "--porcelain"])
            .await?;
        Self::require_success(&output, "status --porcelain")?;
        Ok(Self::stdout(&output))
    }

    pub async fn switch(&self, worktree: &Path, branch: &str) -> Result<()> {
        self.require_clean_worktree(worktree, "switch").await?;
        let output = self.run(worktree, &["switch", branch]).await?;
        if output.status.success() {
            return Ok(());
        }
        let track = format!("origin/{branch}");
        let output = self.run(worktree, &["switch", "--track", &track]).await?;
        Self::require_success(&output, "switch --track")
    }

    pub async fn switch_new_branch(
        &self,
        worktree: &Path,
        branch: &str,
        start_point: &str,
    ) -> Result<()> {
        self.require_clean_worktree(worktree, "switch").await?;
        let output = self
            .run(worktree, &["switch", "-c", branch, start_point])
            .await?;
        Self::require_success(&output, "switch -c")
    }

    async fn require_clean_worktree(&self, worktree: &Path, action: &str) -> Result<()> {
        let status = self.status_porcelain(worktree).await?;
        if !status.trim().is_empty() {
            return Err(Error::Git(format!(
                "{action} refused: commit or stash uncommitted changes first"
            )));
        }
        Ok(())
    }

    /// Merges `base` (as [`Self::freshest_ref`] picks it) into the checked-out branch.
    /// Stopping on conflicts is `Error::GitConflict`, with the merge left in progress.
    pub async fn merge_base(&self, worktree: &Path, base: &str) -> Result<()> {
        self.require_clean_worktree(worktree, "merge").await?;
        let rev = self.freshest_ref(worktree, base).await;
        let output = self.run(worktree, &["merge", &rev]).await?;
        if output.status.success() {
            return Ok(());
        }
        let paths = self.unmerged_paths(worktree).await?;
        if !paths.is_empty() {
            return Err(Error::GitConflict { paths });
        }
        Err(Self::fail(&output, "merge"))
    }

    /// The paths a merge in progress is stopped on; `None` once it is committed, aborted, or
    /// has every path resolved.
    pub async fn merge_conflict(&self, worktree: &Path) -> Result<Option<GitConflictState>> {
        if !merging(worktree) {
            return Ok(None);
        }
        let paths = self.unmerged_paths(worktree).await?;
        Ok((!paths.is_empty()).then_some(GitConflictState { paths }))
    }

    /// With no merge in progress, as when it was finished or aborted outside Cormux, there is
    /// nothing to abort and that is the outcome wanted.
    pub async fn abort_merge(&self, worktree: &Path) -> Result<()> {
        if !merging(worktree) {
            return Ok(());
        }
        let output = self.run(worktree, &["merge", "--abort"]).await?;
        Self::require_success(&output, "merge --abort")
    }

    /// Local branch tips, newest commit first. `committed` is git's relative date.
    pub async fn list_local_branch_tips(&self, repo: &Path) -> Result<Vec<LocalBranchTip>> {
        let output = self
            .run(
                repo,
                &[
                    "for-each-ref",
                    "--sort=-committerdate",
                    "--format=%(refname:short)%00%(committerdate:relative)%00%(subject)",
                    "refs/heads",
                ],
            )
            .await?;
        Self::require_success(&output, "for-each-ref heads")?;
        Ok(parse_local_branch_tips(&Self::stdout(&output)))
    }

    /// Whether `origin/<branch>` exists and points where the local branch does, as of the
    /// last fetch, so deleting the local branch loses nothing.
    pub async fn in_sync_with_origin(&self, repo: &Path, branch: &str) -> Result<bool> {
        let local = self
            .resolve_ref(repo, &format!("refs/heads/{branch}"))
            .await?;
        let remote = self
            .resolve_ref(repo, &format!("refs/remotes/origin/{branch}"))
            .await?;
        Ok(local.is_some() && local == remote)
    }

    pub async fn list_local_branches(&self, repo: &Path) -> Result<Vec<String>> {
        let output = self
            .run(repo, &["for-each-ref", "--format=%(refname)", "refs/heads"])
            .await?;
        Self::require_success(&output, "for-each-ref heads")?;
        Ok(Self::stdout(&output)
            .lines()
            .filter(|line| !line.is_empty())
            .map(|refname| {
                refname
                    .strip_prefix("refs/heads/")
                    .unwrap_or(refname)
                    .to_string()
            })
            .collect())
    }

    /// Branches on `remote`, without the `<remote>/` prefix or its `HEAD` pointer.
    pub async fn list_remote_branches(&self, repo: &Path, remote: &str) -> Result<Vec<String>> {
        let prefix = format!("refs/remotes/{remote}/");
        let output = self
            .run(repo, &["for-each-ref", "--format=%(refname)", &prefix])
            .await?;
        Self::require_success(&output, "for-each-ref remotes")?;
        Ok(Self::stdout(&output)
            .lines()
            .filter_map(|refname| refname.strip_prefix(prefix.as_str()))
            .filter(|name| !name.is_empty() && *name != "HEAD")
            .map(str::to_string)
            .collect())
    }

    pub async fn unmerged_paths(&self, worktree: &Path) -> Result<Vec<String>> {
        let output = self
            .run(worktree, &["diff", "--name-only", "-z", "--diff-filter=U"])
            .await?;
        Self::diff_ok(&output, "unmerged paths")?;
        // `-z` so non-ASCII and odd paths come back as-is, not C-quoted.
        Ok(Self::stdout(&output)
            .split('\0')
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect())
    }

    pub async fn push(&self, worktree: &Path, branch: &str) -> Result<()> {
        let output = self
            .run(worktree, &["push", "-u", "origin", branch])
            .await?;
        Self::require_success(&output, "push")
    }

    pub async fn branch_delete(&self, repo: &Path, branch: &str) -> Result<()> {
        let output = self.run(repo, &["branch", "-D", branch]).await?;
        Self::require_success(&output, "branch -D")
    }

    /// A range git can't resolve is an error, not 0.
    pub async fn rev_list_count(&self, worktree: &Path, range: &str) -> Result<u32> {
        self.count_commits(worktree, &["rev-list", "--count", range])
            .await
    }

    async fn count_commits(&self, worktree: &Path, args: &[&str]) -> Result<u32> {
        let output = self.run(worktree, args).await?;
        Self::require_success(&output, "rev-list --count")?;
        Self::stdout(&output)
            .trim()
            .parse()
            .map_err(|error| Error::Git(format!("rev-list --count: {error}")))
    }

    /// Commits only `left` has and only `right` has, as `(ahead, behind)` from `left`'s point of
    /// view, in one process. `(0, 0)` when either ref is missing.
    pub async fn ahead_behind(
        &self,
        worktree: &Path,
        left: &str,
        right: &str,
    ) -> Result<(u32, u32)> {
        let range = format!("{left}...{right}");
        let output = self
            .run(worktree, &["rev-list", "--left-right", "--count", &range])
            .await?;
        if !output.status.success() {
            return Ok((0, 0));
        }
        parse_left_right(&Self::stdout(&output))
            .ok_or_else(|| Error::Git("rev-list --left-right: unexpected output".into()))
    }

    pub async fn remote_branch_exists(&self, repo: &Path, branch: &str) -> Result<bool> {
        let reference = format!("refs/remotes/origin/{branch}");
        let output = self
            .run(repo, &["show-ref", "--verify", &reference])
            .await?;
        Ok(output.status.success())
    }

    /// Commits on HEAD that neither a remote nor `base` has, so removing the branch loses
    /// them, pushed or not.
    pub async fn commits_only_here(&self, worktree: &Path, base: &str) -> Result<u32> {
        let mut args = vec!["rev-list", "--count", "HEAD", "--not", "--remotes"];
        if self.ref_exists(worktree, base).await? {
            args.push(base);
        }
        self.count_commits(worktree, &args).await
    }

    pub async fn current_branch(&self, worktree: &Path) -> Result<String> {
        let output = self.run(worktree, &["branch", "--show-current"]).await?;
        Self::require_success(&output, "branch --show-current")?;
        Ok(Self::stdout(&output).trim().to_string())
    }

    /// The worktree's own git dir and the repo's common one (the same for the main checkout).
    pub async fn git_dirs(&self, worktree: &Path) -> Result<(PathBuf, PathBuf)> {
        let output = self
            .run(
                worktree,
                &[
                    "rev-parse",
                    "--path-format=absolute",
                    "--git-dir",
                    "--git-common-dir",
                ],
            )
            .await?;
        Self::require_success(&output, "rev-parse --git-dir")?;
        let stdout = Self::stdout(&output);
        let mut lines = stdout.lines().map(|line| PathBuf::from(line.trim()));
        match (lines.next(), lines.next()) {
            (Some(own), Some(common)) => Ok((own, common)),
            _ => Err(Error::Git("rev-parse --git-dir: unexpected output".into())),
        }
    }

    pub async fn config_get(&self, worktree: &Path, key: &str) -> Result<Option<String>> {
        let output = self.run(worktree, &["config", "--get", key]).await?;
        match output.status.code() {
            Some(0) => Ok(Some(Self::stdout(&output).trim().to_string())),
            // git's "key not set".
            Some(1) => Ok(None),
            _ => Err(Self::fail(&output, "config --get")),
        }
    }

    pub async fn ref_exists(&self, worktree: &Path, reference: &str) -> Result<bool> {
        let output = self
            .run(worktree, &["rev-parse", "--verify", "--quiet", reference])
            .await?;
        Ok(output.status.success())
    }

    pub async fn is_ancestor(&self, worktree: &Path, ancestor: &str, descendant: &str) -> bool {
        self.run(
            worktree,
            &["merge-base", "--is-ancestor", ancestor, descendant],
        )
        .await
        .is_ok_and(|output| output.status.success())
    }

    /// The ref that stands for `branch` wherever a workspace is compared with or brought up to
    /// its base (behind counts, pull, Changes, new branches): `origin/<branch>` when it
    /// has everything the local branch has (a stale local trunk), else the local branch (a
    /// stack branch rebased but not pushed yet).
    pub async fn freshest_ref(&self, worktree: &Path, branch: &str) -> String {
        let remote = format!("origin/{branch}");
        if !self.ref_exists(worktree, &remote).await.unwrap_or(false) {
            return branch.to_string();
        }
        if !self.ref_exists(worktree, branch).await.unwrap_or(false) {
            return remote;
        }
        if self.is_ancestor(worktree, branch, &remote).await {
            remote
        } else {
            branch.to_string()
        }
    }

    pub async fn shortstat(&self, worktree: &Path, range: &str) -> Result<ShortStat> {
        let output = self.run(worktree, &["diff", "--shortstat", range]).await?;
        Self::require_success(&output, "diff --shortstat")?;
        Ok(parse_shortstat(&Self::stdout(&output)))
    }
}

/// Pairs each file's stat with its patch, cut from `whole` (one diff of every file). A file
/// whose patch can't be found there is diffed on its own by `file_patch`.
async fn with_patches_from<F, Fut>(
    stats: Vec<FileStat>,
    whole: &str,
    file_patch: F,
) -> Result<Vec<(FileStat, String)>>
where
    F: Fn(Vec<String>) -> Fut,
    Fut: std::future::Future<Output = Result<String>>,
{
    let mut patches = split_patches(whole);
    let mut changes = Vec::with_capacity(stats.len());
    for stat in stats {
        let patch = match patches.remove(stat.patch_header().as_str()) {
            Some(patch) => patch.to_string(),
            None => file_patch(stat.paths()).await?,
        };
        changes.push((stat, patch));
    }
    Ok(changes)
}

/// `git rev-list --left-right --count` prints `left<TAB>right`.
fn parse_left_right(text: &str) -> Option<(u32, u32)> {
    let mut parts = text.split_whitespace();
    let left = parts.next()?.parse().ok()?;
    let right = parts.next()?.parse().ok()?;
    Some((left, right))
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ShortStat {
    pub files: u32,
    pub added: u32,
    pub deleted: u32,
}

fn parse_shortstat(text: &str) -> ShortStat {
    let mut stat = ShortStat::default();
    for part in text.split(',') {
        let part = part.trim();
        let count = part
            .split_whitespace()
            .next()
            .and_then(|n| n.parse().ok())
            .unwrap_or(0);
        if part.contains("file") {
            stat.files = count;
        } else if part.contains("insertion") {
            stat.added = count;
        } else if part.contains("deletion") {
            stat.deleted = count;
        }
    }
    stat
}

/// One changed file from `git diff --numstat`. A rename keeps the path it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStat {
    pub path: String,
    pub old_path: Option<String>,
    pub added: u32,
    pub deleted: u32,
}

impl FileStat {
    /// Pathspecs for this file: a rename needs both sides, to pair them in a diff or to undo it.
    pub fn paths(&self) -> Vec<String> {
        self.old_path.iter().chain([&self.path]).cloned().collect()
    }

    /// The first line of this file's section in a `PATCH_ARGS` diff. Git quotes the paths it
    /// must (tabs, newlines), so those won't match and are diffed on their own.
    fn patch_header(&self) -> String {
        let old = self.old_path.as_deref().unwrap_or(&self.path);
        format!("diff --git a/{old} b/{}", self.path)
    }
}

/// A whole `git diff` by file, keyed by each section's `diff --git` line. Patch content always
/// starts with a diff marker, so that line can't be mistaken for it.
fn split_patches(diff: &str) -> HashMap<&str, &str> {
    let mut starts: Vec<usize> = diff
        .match_indices("\ndiff --git ")
        .map(|(index, _)| index + 1)
        .collect();
    if diff.starts_with("diff --git ") {
        starts.insert(0, 0);
    }
    let ends = starts.iter().skip(1).copied().chain([diff.len()]);
    starts
        .iter()
        .zip(ends)
        .map(|(&start, end)| {
            let section = &diff[start..end];
            (section.lines().next().unwrap_or_default(), section)
        })
        .collect()
}

/// `.git` of a worktree: the folder itself, or where a linked worktree's `.git` file points.
pub fn worktree_git_dir(worktree: &Path) -> Option<PathBuf> {
    let dot_git = worktree.join(".git");
    if dot_git.is_dir() {
        return Some(dot_git);
    }
    let text = std::fs::read_to_string(&dot_git).ok()?;
    let dir = text.trim().strip_prefix("gitdir:")?;
    Some(worktree.join(dir.trim()))
}

/// The branch a worktree has checked out, read from `HEAD` without spawning git: empty while
/// detached, `None` when `HEAD` can't say (unreadable, or a reftable repo's placeholder).
pub fn checked_out_branch(worktree: &Path) -> Option<String> {
    let head = std::fs::read_to_string(worktree_git_dir(worktree)?.join("HEAD")).ok()?;
    parse_head(&head)
}

/// Whether a merge is in progress in the worktree, read from its `MERGE_HEAD` without spawning
/// git.
pub fn merging(worktree: &Path) -> bool {
    worktree_git_dir(worktree).is_some_and(|dir| dir.join("MERGE_HEAD").is_file())
}

fn parse_head(head: &str) -> Option<String> {
    let Some(reference) = head.trim().strip_prefix("ref:") else {
        return Some(String::new());
    };
    let branch = reference.trim().strip_prefix("refs/heads/")?;
    (branch != ".invalid").then(|| branch.to_string())
}

/// `git diff --numstat -z`: `added\tdeleted\tpath\0`, or for a rename
/// `added\tdeleted\t\0old\0new\0`. Binary files count as no lines.
fn parse_numstat_z(bytes: &[u8]) -> Vec<FileStat> {
    let text = String::from_utf8_lossy(bytes);
    let mut fields = text.split('\0');
    let mut files = Vec::new();
    while let Some(record) = fields.next() {
        let mut parts = record.splitn(3, '\t');
        let (Some(added), Some(deleted), Some(path)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let (path, old_path) = if path.is_empty() {
            let (Some(old), Some(new)) = (fields.next(), fields.next()) else {
                break;
            };
            (new.to_string(), Some(old.to_string()))
        } else {
            (path.to_string(), None)
        };
        files.push(FileStat {
            path,
            old_path,
            added: added.parse().unwrap_or(0),
            deleted: deleted.parse().unwrap_or(0),
        });
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell_env::ShellEnv;

    #[test]
    fn ref_names_that_git_would_read_as_options_are_rejected() {
        for bad in [
            "",
            "-",
            "--upload-pack=touch /tmp/x",
            "--detach",
            "feat/../x",
            "..",
            "feat x",
            "a:b",
            "x.lock",
            "feat/.hidden",
            "a@{1}",
        ] {
            assert!(
                validate_ref_name(bad, "branch").is_err(),
                "{bad:?} accepted"
            );
        }
        for good in ["main", "feat/login-flow", "release/1.2", "cormux/review-12"] {
            assert!(
                validate_ref_name(good, "branch").is_ok(),
                "{good:?} rejected"
            );
        }
    }

    #[test]
    fn parses_local_branch_tips() {
        let raw = "feat/a\u{0}2 hours ago\u{0}Add auth\nmain\u{0}3 days ago\u{0}Initial\n";
        let rows = parse_local_branch_tips(raw);
        assert_eq!(
            rows,
            vec![
                LocalBranchTip {
                    name: "feat/a".into(),
                    committed: "2 hours ago".into(),
                    subject: "Add auth".into(),
                },
                LocalBranchTip {
                    name: "main".into(),
                    committed: "3 days ago".into(),
                    subject: "Initial".into(),
                },
            ]
        );
    }

    /// Git with the test process's own environment, no login shell needed.
    async fn git_with_env() -> Git {
        Git::new(Arc::new(RwLock::new(ShellEnv::new())))
    }

    fn init_repo() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_path_buf();
        run_ok(&path, &["git", "init", "-b", "main"]);
        run_ok(&path, &["git", "config", "user.email", "cormux@test"]);
        run_ok(&path, &["git", "config", "user.name", "Cormux"]);
        std::fs::write(path.join("README.md"), "hello\n").unwrap();
        run_ok(&path, &["git", "add", "README.md"]);
        run_ok(&path, &["git", "commit", "-m", "init"]);
        (dir, path)
    }

    fn run_ok(cwd: &Path, args: &[&str]) {
        let status = std::process::Command::new(args[0])
            .args(&args[1..])
            .current_dir(cwd)
            .status()
            .unwrap();
        assert!(status.success(), "{args:?} failed in {}", cwd.display());
    }

    #[test]
    fn reads_the_branch_from_head() {
        assert_eq!(
            parse_head("ref: refs/heads/feat/a\n").as_deref(),
            Some("feat/a")
        );
        assert_eq!(parse_head("3f2a9c\n").as_deref(), Some(""));
        assert_eq!(parse_head("ref: refs/heads/.invalid\n"), None);
    }

    #[tokio::test]
    async fn a_linked_worktrees_head_is_read_without_git() {
        let git = git_with_env().await;
        let (_dir, repo) = init_repo();
        assert_eq!(checked_out_branch(&repo).as_deref(), Some("main"));
        let dir = tempfile::tempdir().unwrap();
        let worktree = dir.path().join("wt");
        git.worktree_add(&repo, &worktree, "feat/x", "main")
            .await
            .unwrap();
        assert_eq!(checked_out_branch(&worktree).as_deref(), Some("feat/x"));
    }

    /// Every kind of change git treats specially, each compared with diffing that file alone.
    #[tokio::test]
    async fn the_whole_diff_splits_into_what_each_file_would_show() {
        let git = git_with_env().await;
        let (_dir, repo) = init_repo();
        let long: String = (0..40).map(|n| format!("line {n}\n")).collect();
        for (name, body) in [
            ("to-rename.txt", long.as_str()),
            ("to-delete.txt", "gone\n"),
            ("spaces in name.txt", "a\n"),
            ("caf\u{e9}.txt", "a\n"),
            ("tab\tname.txt", "a\n"),
            ("script.sh", "#!/bin/sh\n"),
        ] {
            std::fs::write(repo.join(name), body).unwrap();
        }
        std::fs::write(repo.join("logo.bin"), [0u8, 1, 2]).unwrap();
        run_ok(&repo, &["git", "add", "."]);
        run_ok(&repo, &["git", "commit", "-qm", "files"]);

        std::fs::rename(repo.join("to-rename.txt"), repo.join("renamed.txt")).unwrap();
        std::fs::remove_file(repo.join("to-delete.txt")).unwrap();
        std::fs::write(repo.join("spaces in name.txt"), "b\n").unwrap();
        std::fs::write(repo.join("caf\u{e9}.txt"), "b\n").unwrap();
        std::fs::write(repo.join("tab\tname.txt"), "b\n").unwrap();
        std::fs::write(repo.join("logo.bin"), [3u8, 4, 5]).unwrap();
        std::fs::write(repo.join("untracked.txt"), "new\n").unwrap();
        run_ok(&repo, &["chmod", "+x", "script.sh"]);

        let changes = git.uncommitted_changes(&repo, true).await.unwrap();
        assert_eq!(changes.len(), 8);
        let index = git.scratch_index(&repo).await.unwrap();
        let index = index.to_string_lossy().to_string();
        let env = [("GIT_INDEX_FILE", index.as_str())];
        git.run_with_env(Some(&repo), &["add", "--intent-to-add", "--", "."], &env)
            .await
            .unwrap();
        for (stat, patch) in &changes {
            let mut args = PATCH_ARGS.to_vec();
            args.extend(["HEAD", "--"]);
            let paths = stat.paths();
            args.extend(paths.iter().map(String::as_str));
            let alone = git.run_with_env(Some(&repo), &args, &env).await.unwrap();
            assert_eq!(*patch, Git::stdout(&alone), "{}", stat.path);
            assert!(!patch.is_empty(), "{}", stat.path);
        }
        let _ = std::fs::remove_file(&index);

        run_ok(&repo, &["git", "switch", "-qc", "feat/a"]);
        run_ok(&repo, &["git", "add", "."]);
        run_ok(&repo, &["git", "commit", "-qm", "everything"]);
        let committed = git.range_changes(&repo, "main...feat/a").await.unwrap();
        assert_eq!(committed.len(), 8);
        for (stat, patch) in &committed {
            assert!(!patch.is_empty(), "{}", stat.path);
        }
    }

    /// Bug: behind counts and pull/rebase used `origin/<base>` while Changes used the freshest
    /// ref, so a branch stacked on an unpushed local base counted nothing to pull (or failed).
    #[tokio::test]
    async fn behind_and_pull_use_the_same_base_as_changes() {
        let git = git_with_env().await;
        let (_origin_dir, origin) = init_repo();
        let (_clone_dir, clone) = clone_of(&origin);
        run_ok(&clone, &["git", "switch", "-qc", "feat/a"]);
        commit(&clone, "a1.txt");
        run_ok(&clone, &["git", "switch", "-qc", "feat/b"]);
        run_ok(&clone, &["git", "switch", "-q", "feat/a"]);
        commit(&clone, "a2.txt");
        run_ok(&clone, &["git", "switch", "-q", "feat/b"]);

        assert_eq!(git.freshest_ref(&clone, "feat/a").await, "feat/a");
        assert_eq!(git.behind_count(&clone, "feat/a").await.unwrap(), 1);
        git.merge_base(&clone, "feat/a").await.unwrap();
        assert_eq!(git.behind_count(&clone, "feat/a").await.unwrap(), 0);
        // A stale local trunk still counts against origin's.
        commit(&origin, "landed.txt");
        git.fetch(&clone).await.unwrap();
        assert_eq!(git.behind_count(&clone, "main").await.unwrap(), 1);
    }

    #[tokio::test]
    async fn base_fetches_are_shared_only_when_they_worked() {
        let git = git_with_env().await;
        let (_origin_dir, origin) = init_repo();
        let (_clone_dir, clone) = clone_of(&origin);
        assert_eq!(git.latest_base(&clone, "main").await, "origin/main");
        // Inside the window a new commit waits for the next fetch.
        commit(&origin, "landed.txt");
        git.latest_base(&clone, "main").await;
        assert!(!git.ref_has(&clone, "origin/main", "landed.txt").await);

        let (_missing_dir, missing) = clone_of(&origin);
        run_ok(
            &missing,
            &["git", "remote", "set-url", "origin", "/nowhere"],
        );
        git.latest_base(&missing, "main").await;
        let origin_path = origin.to_string_lossy().to_string();
        run_ok(
            &missing,
            &["git", "remote", "set-url", "origin", &origin_path],
        );
        commit(&origin, "later.txt");
        git.latest_base(&missing, "main").await;
        assert!(git.ref_has(&missing, "origin/main", "later.txt").await);
    }

    #[tokio::test]
    async fn a_count_git_cannot_make_is_an_error_not_zero() {
        let git = git_with_env().await;
        let (_dir, repo) = init_repo();
        assert!(git.rev_list_count(&repo, "nope..HEAD").await.is_err());
        assert_eq!(git.config_get(&repo, "cormux.unset").await.unwrap(), None);
        let (own, common) = git.git_dirs(&repo).await.unwrap();
        assert_eq!(own, common);
    }

    #[tokio::test]
    async fn an_idle_worktree_leaves_no_queue_behind() {
        let git = git_with_env().await;
        let (_dir, repo) = init_repo();
        git.status_porcelain(&repo).await.unwrap();
        assert!(git.inner.queue.is_empty());
    }

    #[tokio::test]
    async fn shortstat_counts_lines_between_refs() {
        let git = git_with_env().await;
        let (_dir, repo) = init_repo();
        run_ok(&repo, &["git", "switch", "-c", "feat/a.v1"]);
        std::fs::write(repo.join("README.md"), "hello\nworld\nagain\n").unwrap();
        run_ok(&repo, &["git", "commit", "-am", "more"]);
        let stat = git.shortstat(&repo, "main...feat/a.v1").await.unwrap();
        assert_eq!((stat.files, stat.added, stat.deleted), (1, 2, 0));
        assert!(git.ref_exists(&repo, "feat/a.v1").await.unwrap());
        assert!(!git.ref_exists(&repo, "origin/main").await.unwrap());
    }

    #[tokio::test]
    async fn counts_ahead_and_behind_in_one_call() {
        let git = git_with_env().await;
        let (_origin_dir, origin) = init_repo();
        let (_clone_dir, clone) = clone_of(&origin);
        assert_eq!(
            git.ahead_behind(&clone, "main", "origin/main")
                .await
                .unwrap(),
            (0, 0)
        );
        commit(&clone, "local.txt");
        commit(&origin, "remote-1.txt");
        commit(&origin, "remote-2.txt");
        run_ok(&clone, &["git", "fetch", "-q", "origin"]);
        assert_eq!(
            git.ahead_behind(&clone, "main", "origin/main")
                .await
                .unwrap(),
            (1, 2)
        );
        assert_eq!(
            git.ahead_behind(&clone, "main", "origin/nope")
                .await
                .unwrap(),
            (0, 0)
        );
        assert_eq!(parse_left_right("3\t4\n"), Some((3, 4)));
        assert_eq!(parse_left_right(""), None);
    }

    #[test]
    fn parses_shortstat() {
        assert_eq!(
            parse_shortstat(" 3 files changed, 10 insertions(+), 2 deletions(-)\n"),
            ShortStat {
                files: 3,
                added: 10,
                deleted: 2
            }
        );
        assert_eq!(
            parse_shortstat(" 1 file changed, 1 deletion(-)"),
            ShortStat {
                files: 1,
                added: 0,
                deleted: 1
            }
        );
        assert_eq!(parse_shortstat(""), ShortStat::default());
    }

    #[test]
    fn parses_renames_in_numstat() {
        let raw = b"1\t0\tkeep.txt\x000\t0\t\x00src/one.txt\x00src/two.txt\x00-\t-\tlogo.png\x00";
        assert_eq!(
            parse_numstat_z(raw),
            vec![
                FileStat {
                    path: "keep.txt".into(),
                    old_path: None,
                    added: 1,
                    deleted: 0,
                },
                FileStat {
                    path: "src/two.txt".into(),
                    old_path: Some("src/one.txt".into()),
                    added: 0,
                    deleted: 0,
                },
                FileStat {
                    path: "logo.png".into(),
                    old_path: None,
                    added: 0,
                    deleted: 0,
                },
            ]
        );
    }

    #[tokio::test]
    async fn a_moved_file_is_one_change_and_the_index_is_left_alone() {
        let git = git_with_env().await;
        let (_dir, repo) = init_repo();
        std::fs::create_dir(repo.join("docs")).unwrap();
        std::fs::rename(repo.join("README.md"), repo.join("docs/README.md")).unwrap();
        std::fs::write(repo.join("new.txt"), "one\ntwo\n").unwrap();

        let changes = git.uncommitted_changes(&repo, true).await.unwrap();
        let stats: Vec<_> = changes.iter().map(|(stat, _)| stat.clone()).collect();
        assert_eq!(
            stats,
            vec![
                FileStat {
                    path: "docs/README.md".into(),
                    old_path: Some("README.md".into()),
                    added: 0,
                    deleted: 0,
                },
                FileStat {
                    path: "new.txt".into(),
                    old_path: None,
                    added: 2,
                    deleted: 0,
                },
            ]
        );
        assert!(changes[0].1.contains("rename to docs/README.md"));
        assert!(changes[1].1.contains("+two"));
        // Nothing was staged in the real index.
        assert_eq!(
            git.status_porcelain(&repo).await.unwrap(),
            " D README.md\n?? docs/\n?? new.txt\n"
        );
    }

    #[tokio::test]
    async fn discards_some_or_all_uncommitted_changes() {
        let git = git_with_env().await;
        let (_dir, repo) = init_repo();
        std::fs::write(repo.join("README.md"), "changed\n").unwrap();
        std::fs::write(repo.join("new.txt"), "new\n").unwrap();
        std::fs::write(repo.join("staged.txt"), "staged\n").unwrap();
        std::fs::write(repo.join("other.txt"), "other\n").unwrap();
        run_ok(&repo, &["git", "add", "staged.txt"]);

        git.discard(
            &repo,
            &["README.md".into(), "new.txt".into(), "staged.txt".into()],
        )
        .await
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(repo.join("README.md")).unwrap(),
            "hello\n"
        );
        assert_eq!(git.status_porcelain(&repo).await.unwrap(), "?? other.txt\n");

        std::fs::create_dir(repo.join("dir")).unwrap();
        std::fs::write(repo.join("dir/file.txt"), "x\n").unwrap();
        std::fs::remove_file(repo.join("README.md")).unwrap();
        git.discard(&repo, &[]).await.unwrap();
        assert_eq!(git.status_porcelain(&repo).await.unwrap(), "");
        assert!(!repo.join("dir").exists());
    }

    fn commit(repo: &Path, file: &str) {
        std::fs::write(repo.join(file), "x\n").unwrap();
        run_ok(repo, &["git", "add", file]);
        run_ok(repo, &["git", "commit", "-q", "-m", file]);
    }

    impl Git {
        async fn ref_has(&self, repo: &Path, reference: &str, file: &str) -> bool {
            let spec = format!("{reference}:{file}");
            self.ref_exists(repo, &spec).await.unwrap()
        }
    }

    fn clone_of(origin: &Path) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        run_ok(
            dir.path(),
            &["git", "clone", "-q", &origin.to_string_lossy(), "clone"],
        );
        let clone = dir.path().join("clone");
        run_ok(&clone, &["git", "config", "user.email", "cormux@test"]);
        run_ok(&clone, &["git", "config", "user.name", "Cormux"]);
        (dir, clone)
    }

    #[tokio::test]
    async fn a_new_branch_starts_from_what_origin_just_got() {
        let git = git_with_env().await;
        let (_origin_dir, origin) = init_repo();
        let (_clone_dir, clone) = clone_of(&origin);
        // Local main is now behind, and nothing has fetched since.
        commit(&origin, "landed.txt");

        let start = git.latest_base(&clone, "main").await;
        assert_eq!(start, "origin/main");

        let worktree = clone.parent().unwrap().join("wt");
        git.worktree_add(&clone, &worktree, "feat/new", &start)
            .await
            .unwrap();
        assert!(worktree.join("landed.txt").exists());
        // It doesn't track main, so a push can't land there.
        let upstream = std::process::Command::new("git")
            .args(["config", "branch.feat/new.remote"])
            .current_dir(&clone)
            .output()
            .unwrap();
        assert!(!upstream.status.success());
    }

    #[tokio::test]
    async fn a_new_branch_keeps_commits_origin_does_not_have() {
        let git = git_with_env().await;
        let (_origin_dir, origin) = init_repo();
        let (_clone_dir, clone) = clone_of(&origin);
        commit(&clone, "unpushed.txt");
        assert_eq!(git.latest_base(&clone, "main").await, "main");
    }

    #[tokio::test]
    async fn a_new_branch_falls_back_when_it_cannot_fetch() {
        let git = git_with_env().await;
        let (_origin_dir, origin) = init_repo();
        let (_clone_dir, clone) = clone_of(&origin);
        run_ok(&clone, &["git", "remote", "set-url", "origin", "/nowhere"]);
        // Its last fetch is what it knows: origin/main, level with main.
        assert_eq!(git.latest_base(&clone, "main").await, "origin/main");

        let (_dir, alone) = init_repo();
        assert_eq!(git.latest_base(&alone, "main").await, "main");
    }

    #[tokio::test]
    async fn counts_commits_only_this_branch_has_even_if_it_was_never_pushed() {
        let git = git_with_env().await;
        let (_origin_dir, origin) = init_repo();
        let (_clone_dir, clone) = clone_of(&origin);
        run_ok(&clone, &["git", "switch", "-q", "-c", "feat/new"]);
        assert_eq!(git.commits_only_here(&clone, "main").await.unwrap(), 0);

        commit(&clone, "one.txt");
        commit(&clone, "two.txt");
        assert_eq!(git.commits_only_here(&clone, "main").await.unwrap(), 2);

        run_ok(&clone, &["git", "push", "-q", "-u", "origin", "feat/new"]);
        assert_eq!(git.commits_only_here(&clone, "main").await.unwrap(), 0);
        commit(&clone, "three.txt");
        assert_eq!(git.commits_only_here(&clone, "main").await.unwrap(), 1);

        // No remote at all: the base still counts as somewhere else.
        let (_dir, alone) = init_repo();
        run_ok(&alone, &["git", "switch", "-q", "-c", "feat/new"]);
        commit(&alone, "one.txt");
        assert_eq!(git.commits_only_here(&alone, "main").await.unwrap(), 1);
    }

    #[tokio::test]
    async fn a_branch_is_in_sync_only_when_origin_has_the_same_tip() {
        let git = git_with_env().await;
        let (_origin_dir, origin) = init_repo();
        run_ok(&origin, &["git", "branch", "feat/pushed"]);
        let clone_dir = tempfile::tempdir().unwrap();
        let clone = clone_dir.path().join("clone");
        run_ok(
            clone_dir.path(),
            &["git", "clone", "-q", &origin.to_string_lossy(), "clone"],
        );
        run_ok(&clone, &["git", "config", "user.email", "cormux@test"]);
        run_ok(&clone, &["git", "config", "user.name", "Cormux"]);
        run_ok(
            &clone,
            &["git", "branch", "feat/pushed", "origin/feat/pushed"],
        );
        run_ok(&clone, &["git", "branch", "feat/local"]);
        assert!(
            git.in_sync_with_origin(&clone, "feat/pushed")
                .await
                .unwrap()
        );
        assert!(!git.in_sync_with_origin(&clone, "feat/local").await.unwrap());

        run_ok(&clone, &["git", "switch", "-q", "feat/pushed"]);
        std::fs::write(clone.join("more.txt"), "more\n").unwrap();
        run_ok(&clone, &["git", "add", "more.txt"]);
        run_ok(&clone, &["git", "commit", "-q", "-m", "more"]);
        assert!(
            !git.in_sync_with_origin(&clone, "feat/pushed")
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn lists_and_switches_to_remote_only_branches() {
        let git = git_with_env().await;
        let (_origin_dir, origin) = init_repo();
        run_ok(&origin, &["git", "branch", "feat/remote-only"]);
        let clone_dir = tempfile::tempdir().unwrap();
        let clone = clone_dir.path().join("clone");
        run_ok(
            clone_dir.path(),
            &["git", "clone", "-q", &origin.to_string_lossy(), "clone"],
        );

        let mut remote = git.list_remote_branches(&clone, "origin").await.unwrap();
        remote.sort();
        assert_eq!(remote, vec!["feat/remote-only", "main"]);
        assert!(
            !git.list_local_branches(&clone)
                .await
                .unwrap()
                .contains(&"feat/remote-only".to_string())
        );

        git.switch(&clone, "feat/remote-only").await.unwrap();
        assert_eq!(
            git.current_branch(&clone).await.unwrap(),
            "feat/remote-only"
        );
    }

    #[tokio::test]
    async fn validates_and_lists_branches() {
        let git = git_with_env().await;
        let (_dir, repo) = init_repo();
        let top = git.show_toplevel(&repo).await.unwrap();
        assert_eq!(top.canonicalize().unwrap(), repo.canonicalize().unwrap());
        let branches = git.list_local_branches(&repo).await.unwrap();
        assert!(branches.iter().any(|branch| branch == "main"));
        assert_eq!(git.default_branch(&repo).await.unwrap(), "main");
    }

    #[tokio::test]
    async fn worktree_queue_numstat_and_switch_lock() {
        let git = git_with_env().await;
        let (_dir, repo) = init_repo();
        let worktree_dir = tempfile::tempdir().unwrap();
        let worktree = worktree_dir.path().join("wt-feature");
        git.worktree_add(&repo, &worktree, "feature", "main")
            .await
            .unwrap();

        std::fs::write(worktree.join("README.md"), "hello\nworld\n").unwrap();
        std::fs::write(worktree.join("extra.txt"), "new\n").unwrap();
        let stats = git.numstat(&worktree).await.unwrap();
        assert!(
            stats
                .iter()
                .any(|stat| stat.path == "README.md" && stat.added > 0)
        );
        assert!(stats.iter().any(|stat| stat.path == "extra.txt"));

        let dirty = git.switch(&worktree, "main").await;
        assert!(dirty.is_err());

        git.worktree_remove(&repo, &worktree, true).await.unwrap();
        git.branch_delete(&repo, "feature").await.unwrap();
    }

    #[tokio::test]
    async fn removing_a_worktree_does_not_wait_for_the_repo_queue() {
        let git = git_with_env().await;
        let (_dir, repo) = init_repo();
        let worktree_dir = tempfile::tempdir().unwrap();
        let worktree = worktree_dir.path().join("wt-feature");
        git.worktree_add(&repo, &worktree, "feature", "main")
            .await
            .unwrap();

        let removal = git.worktree_remove(&repo, &worktree, false);
        let finished = git
            .with_queue(&repo, || {
                tokio::time::timeout(Duration::from_secs(10), removal)
            })
            .await;

        finished
            .expect("removal waited for the repo queue")
            .unwrap();
        assert!(!worktree.exists());
    }

    #[tokio::test]
    async fn a_second_call_on_a_worktree_waits_for_the_first() {
        let git = git_with_env().await;
        let (_dir, repo) = init_repo();
        let overlapped = git
            .with_queue(&repo, || {
                tokio::time::timeout(Duration::from_millis(200), git.status_porcelain(&repo))
            })
            .await;
        assert!(overlapped.is_err(), "ran while the worktree was busy");
        git.status_porcelain(&repo).await.unwrap();
    }
}
