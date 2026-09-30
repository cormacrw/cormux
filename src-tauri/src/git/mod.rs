mod diff;
mod fetch;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::process::Command;
use tokio::sync::{Mutex as AsyncMutex, RwLock};

use crate::error::{Error, Result};
use crate::shell_env::ShellEnv;

pub use diff::{DiffTarget, LiveDiffEngine, WorktreeDiff};
pub use fetch::{BehindUpdate, FetchScheduler, RepoFetchTarget, WorkspaceFetchTarget};

/// System `git` wrapper. Credential helpers, hooks and LFS apply because this
/// shells out instead of using git2/gix. Calls for one worktree are serialised.
#[derive(Clone)]
pub struct Git {
    inner: Arc<GitInner>,
}

struct GitInner {
    env: Arc<RwLock<ShellEnv>>,
    queues: Mutex<HashMap<String, Arc<AsyncMutex<()>>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BranchRef {
    pub name: String,
    pub refname: String,
}

impl Git {
    pub fn new(env: Arc<RwLock<ShellEnv>>) -> Self {
        Self {
            inner: Arc::new(GitInner {
                env,
                queues: Mutex::new(HashMap::new()),
            }),
        }
    }

    fn queue_key(path: &Path) -> String {
        path.to_string_lossy().to_string()
    }

    async fn with_queue<T, F, Fut>(&self, worktree: &Path, f: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let lock = {
            let mut queues = self
                .inner
                .queues
                .lock()
                .map_err(|error| Error::Git(error.to_string()))?;
            queues
                .entry(Self::queue_key(worktree))
                .or_insert_with(|| Arc::new(AsyncMutex::new(())))
                .clone()
        };
        let _guard = lock.lock().await;
        f().await
    }

    async fn run(&self, cwd: &Path, args: &[&str]) -> Result<Output> {
        self.with_queue(cwd, || async { self.run_unlocked(Some(cwd), args).await })
            .await
    }

    /// Run another git-driving tool (e.g. `gh stack`) in `worktree`, queued behind and
    /// ahead of this worktree's own git calls. Spawn failures come back as `Err`.
    pub async fn run_tool(
        &self,
        worktree: &Path,
        program: &str,
        args: &[&str],
        extra_env: &[(&str, &str)],
    ) -> Result<Output> {
        self.with_queue(worktree, || async {
            let env = self.inner.env.read().await;
            let mut command = Command::new(program);
            command.args(args);
            env.apply(&mut command);
            command.envs(extra_env.iter().copied());
            command.current_dir(worktree);
            command.stdin(std::process::Stdio::null());
            command
                .output()
                .await
                .map_err(|error| Error::Git(format!("{program}: {error}")))
        })
        .await
    }

    async fn run_unlocked(&self, cwd: Option<&Path>, args: &[&str]) -> Result<Output> {
        let env = self.inner.env.read().await;
        let mut command = Command::new("git");
        command.args(args);
        env.apply(&mut command);
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        command.stdin(std::process::Stdio::null());
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

    pub async fn validate_repo(&self, path: &str) -> Result<String> {
        self.show_toplevel(Path::new(path))
            .await
            .map(|path| path.to_string_lossy().to_string())
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
                .unwrap_or("main")
                .to_string();
            return Ok(branch);
        }
        Ok("main".into())
    }

    pub async fn list_branches(&self, repo: &Path) -> Result<Vec<BranchRef>> {
        let output = self
            .run(
                repo,
                &[
                    "for-each-ref",
                    "--format=%(refname)",
                    "refs/heads",
                    "refs/remotes",
                ],
            )
            .await?;
        Self::require_success(&output, "for-each-ref")?;
        Ok(Self::stdout(&output)
            .lines()
            .filter(|line| !line.is_empty())
            .map(|refname| BranchRef {
                name: short_ref(refname),
                refname: refname.to_string(),
            })
            .collect())
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
            .run(repo, &["worktree", "add", "-b", branch, &path_str, base])
            .await?;
        Self::require_success(&output, "worktree add")
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

    pub async fn worktree_remove(&self, repo: &Path, path: &Path, force: bool) -> Result<()> {
        let path_str = path.to_string_lossy().to_string();
        let output = if force {
            self.run(repo, &["worktree", "remove", "--force", &path_str])
                .await?
        } else {
            self.run(repo, &["worktree", "remove", &path_str]).await?
        };
        Self::require_success(&output, "worktree remove")
    }

    pub async fn behind_count(&self, worktree: &Path, base: &str) -> Result<u32> {
        let range = format!("HEAD..origin/{base}");
        let output = self.run(worktree, &["rev-list", "--count", &range]).await?;
        Self::require_success(&output, "rev-list --count")?;
        Self::stdout(&output)
            .trim()
            .parse()
            .map_err(|error| Error::Git(format!("behind count: {error}")))
    }

    pub async fn fetch(&self, repo: &Path) -> Result<()> {
        let output = self.run(repo, &["fetch", "--all", "--prune"]).await?;
        Self::require_success(&output, "fetch")
    }

    /// Fast-forward the repo checkout's local `branch` to origin's and return how many
    /// commits it moved. A checked-out `branch` is pulled in place; otherwise only the
    /// ref moves, so the checkout's working tree is left alone.
    pub async fn pull_branch(&self, repo: &Path, branch: &str) -> Result<u32> {
        let local = format!("refs/heads/{branch}");
        let before = self.resolve_ref(repo, &local).await?;
        if self.current_branch(repo).await.ok().as_deref() == Some(branch) {
            let output = self
                .run(repo, &["pull", "--ff-only", "origin", branch])
                .await?;
            Self::require_success(&output, "pull --ff-only")?;
        } else {
            let refspec = format!("{local}:{local}");
            let output = self.run(repo, &["fetch", "origin", &refspec]).await?;
            Self::require_success(&output, "fetch")?;
        }
        let (Some(before), Some(after)) = (before, self.resolve_ref(repo, &local).await?) else {
            return Ok(0);
        };
        let range = format!("{before}..{after}");
        let output = self.run(repo, &["rev-list", "--count", &range]).await?;
        Self::require_success(&output, "rev-list --count")?;
        Self::stdout(&output)
            .trim()
            .parse()
            .map_err(|error| Error::Git(format!("pulled count: {error}")))
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

    pub async fn merge_base(&self, worktree: &Path, base: &str) -> Result<String> {
        let other = format!("origin/{base}");
        let output = self.run(worktree, &["merge-base", "HEAD", &other]).await?;
        if output.status.success() {
            return Ok(Self::stdout(&output).trim().to_string());
        }
        let output = self.run(worktree, &["merge-base", "HEAD", base]).await?;
        Self::require_success(&output, "merge-base")?;
        Ok(Self::stdout(&output).trim().to_string())
    }

    pub async fn numstat(&self, worktree: &Path) -> Result<Vec<(String, u32, u32)>> {
        let output = self.run(worktree, &["diff", "--numstat", "HEAD"]).await?;
        Self::diff_ok(&output, "diff --numstat")?;
        let mut files = parse_numstat(&Self::stdout(&output));
        files.extend(self.untracked_numstat(worktree).await?);
        Ok(files)
    }

    /// Files changed by `range`, e.g. `main...feature`.
    pub async fn numstat_range(
        &self,
        worktree: &Path,
        range: &str,
    ) -> Result<Vec<(String, u32, u32)>> {
        let output = self.run(worktree, &["diff", "--numstat", range]).await?;
        Self::diff_ok(&output, "diff --numstat")?;
        Ok(parse_numstat(&Self::stdout(&output)))
    }

    async fn untracked_numstat(&self, worktree: &Path) -> Result<Vec<(String, u32, u32)>> {
        let output = self
            .run(
                worktree,
                &["ls-files", "-z", "--others", "--exclude-standard"],
            )
            .await?;
        Self::require_success(&output, "ls-files")?;
        let mut files = Vec::new();
        for path in parse_z(&output.stdout) {
            if path.is_empty() {
                continue;
            }
            let added = count_lines(&worktree.join(&path));
            files.push((path, added, 0));
        }
        Ok(files)
    }

    pub async fn diff_text(&self, worktree: &Path, merge_base: Option<&str>) -> Result<String> {
        let output = if let Some(base) = merge_base {
            self.run(worktree, &["diff", base]).await?
        } else {
            self.run(worktree, &["diff", "HEAD"]).await?
        };
        Self::diff_ok(&output, "diff")?;
        let mut text = Self::stdout(&output);
        let untracked = self
            .run(
                worktree,
                &["ls-files", "-z", "--others", "--exclude-standard"],
            )
            .await?;
        Self::require_success(&untracked, "ls-files")?;
        for path in parse_z(&untracked.stdout) {
            if path.is_empty() {
                continue;
            }
            let file = worktree.join(&path);
            let empty = PathBuf::from("/dev/null");
            let empty_str = empty.to_string_lossy().to_string();
            let file_str = file.to_string_lossy().to_string();
            let file_diff = self
                .run_unlocked(
                    Some(worktree),
                    &["diff", "--no-index", "--", &empty_str, &file_str],
                )
                .await?;
            Self::diff_ok(&file_diff, "diff --no-index")?;
            text.push_str(&Self::stdout(&file_diff));
        }
        Ok(text)
    }

    pub async fn diff_file(&self, worktree: &Path, path: &str) -> Result<String> {
        let tracked = self.run(worktree, &["diff", "HEAD", "--", path]).await?;
        Self::diff_ok(&tracked, "diff file")?;
        let text = Self::stdout(&tracked);
        if !text.trim().is_empty() {
            return Ok(text);
        }
        let file = worktree.join(path);
        if file.exists() {
            let empty = PathBuf::from("/dev/null");
            let empty_str = empty.to_string_lossy().to_string();
            let file_str = file.to_string_lossy().to_string();
            let file_diff = self
                .run_unlocked(
                    Some(worktree),
                    &["diff", "--no-index", "--", &empty_str, &file_str],
                )
                .await?;
            Self::diff_ok(&file_diff, "diff --no-index")?;
            return Ok(Self::stdout(&file_diff));
        }
        Ok(text)
    }

    pub async fn diff_file_range(
        &self,
        worktree: &Path,
        range: &str,
        path: &str,
    ) -> Result<String> {
        let output = self.run(worktree, &["diff", range, "--", path]).await?;
        Self::diff_ok(&output, "diff file in range")?;
        Ok(Self::stdout(&output))
    }

    pub async fn status_porcelain(&self, worktree: &Path) -> Result<String> {
        let output = self.run(worktree, &["status", "--porcelain"]).await?;
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

    pub async fn merge(&self, worktree: &Path, base: &str) -> Result<()> {
        self.require_clean_worktree(worktree, "merge").await?;
        let rev = format!("origin/{base}");
        let output = self.run(worktree, &["merge", &rev]).await?;
        if output.status.success() {
            return Ok(());
        }
        let paths = self.unmerged_paths(worktree).await?;
        if !paths.is_empty() {
            return Err(Error::GitConflict {
                operation: "merge".into(),
                paths,
            });
        }
        Err(Self::fail(&output, "merge"))
    }

    pub async fn rebase(&self, worktree: &Path, base: &str) -> Result<()> {
        self.require_clean_worktree(worktree, "rebase").await?;
        let rev = format!("origin/{base}");
        let output = self.run(worktree, &["rebase", &rev]).await?;
        if output.status.success() {
            return Ok(());
        }
        let paths = self.unmerged_paths(worktree).await?;
        if !paths.is_empty() {
            return Err(Error::GitConflict {
                operation: "rebase".into(),
                paths,
            });
        }
        Err(Self::fail(&output, "rebase"))
    }

    pub async fn abort_merge(&self, worktree: &Path) -> Result<()> {
        let output = self.run(worktree, &["merge", "--abort"]).await?;
        Self::require_success(&output, "merge --abort")
    }

    pub async fn abort_rebase(&self, worktree: &Path) -> Result<()> {
        let output = self.run(worktree, &["rebase", "--abort"]).await?;
        Self::require_success(&output, "rebase --abort")
    }

    pub async fn branches_in_worktrees(&self, repo: &Path) -> Result<Vec<String>> {
        let output = self.run(repo, &["worktree", "list", "--porcelain"]).await?;
        Self::require_success(&output, "worktree list")?;
        let mut branches = Vec::new();
        for line in Self::stdout(&output).lines() {
            let Some(rest) = line.strip_prefix("branch ") else {
                continue;
            };
            let name = rest
                .strip_prefix("refs/heads/")
                .unwrap_or(rest)
                .to_string();
            if !name.is_empty() {
                branches.push(name);
            }
        }
        Ok(branches)
    }

    pub async fn list_local_branches(&self, repo: &Path) -> Result<Vec<String>> {
        let output = self
            .run(
                repo,
                &["for-each-ref", "--format=%(refname)", "refs/heads"],
            )
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
            .run(worktree, &["diff", "--name-only", "--diff-filter=U"])
            .await?;
        Self::diff_ok(&output, "unmerged paths")?;
        Ok(Self::stdout(&output)
            .lines()
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

    pub async fn rev_list_count(&self, worktree: &Path, range: &str) -> Result<u32> {
        let output = self.run(worktree, &["rev-list", "--count", range]).await?;
        if !output.status.success() {
            return Ok(0);
        }
        Self::stdout(&output)
            .trim()
            .parse()
            .map_err(|error| Error::Git(format!("rev-list count: {error}")))
    }

    pub async fn remote_branch_exists(&self, repo: &Path, branch: &str) -> Result<bool> {
        let reference = format!("refs/remotes/origin/{branch}");
        let output = self.run(repo, &["show-ref", "--verify", &reference]).await?;
        Ok(output.status.success())
    }

    pub async fn unpushed_commit_count(&self, worktree: &Path, branch: &str) -> Result<u32> {
        let range = format!("origin/{branch}..HEAD");
        self.rev_list_count(worktree, &range).await
    }

    pub async fn current_branch(&self, worktree: &Path) -> Result<String> {
        let output = self.run(worktree, &["branch", "--show-current"]).await?;
        Self::require_success(&output, "branch --show-current")?;
        Ok(Self::stdout(&output).trim().to_string())
    }

    pub async fn ref_exists(&self, worktree: &Path, reference: &str) -> Result<bool> {
        let output = self
            .run(worktree, &["rev-parse", "--verify", "--quiet", reference])
            .await?;
        Ok(output.status.success())
    }

    /// Whether `descendant` already has every commit on `ancestor`.
    pub async fn is_ancestor(&self, worktree: &Path, ancestor: &str, descendant: &str) -> bool {
        self.run(
            worktree,
            &["merge-base", "--is-ancestor", ancestor, descendant],
        )
        .await
        .is_ok_and(|output| output.status.success())
    }

    /// `origin/<branch>` when it has everything the local branch has (a stale local
    /// trunk), else the local branch (a stack branch rebased but not pushed yet).
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

    /// Lines added and removed by `range` (e.g. `main...feature`).
    pub async fn shortstat(&self, worktree: &Path, range: &str) -> Result<ShortStat> {
        let output = self.run(worktree, &["diff", "--shortstat", range]).await?;
        Self::require_success(&output, "diff --shortstat")?;
        Ok(parse_shortstat(&Self::stdout(&output)))
    }
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

fn short_ref(refname: &str) -> String {
    refname
        .strip_prefix("refs/heads/")
        .or_else(|| refname.strip_prefix("refs/remotes/"))
        .unwrap_or(refname)
        .to_string()
}

fn parse_numstat(text: &str) -> Vec<(String, u32, u32)> {
    let mut files = Vec::new();
    for line in text.lines() {
        let mut parts = line.splitn(3, '\t');
        let added = parts.next().unwrap_or("-");
        let deleted = parts.next().unwrap_or("-");
        let path = parts.next().unwrap_or("");
        if path.is_empty() {
            continue;
        }
        let added = added.parse().unwrap_or(0);
        let deleted = deleted.parse().unwrap_or(0);
        files.push((path.to_string(), added, deleted));
    }
    files
}

fn parse_z(bytes: &[u8]) -> Vec<String> {
    bytes
        .split(|b| *b == 0)
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| String::from_utf8_lossy(chunk).to_string())
        .collect()
}

fn count_lines(path: &Path) -> u32 {
    std::fs::read_to_string(path)
        .map(|text| text.lines().count() as u32)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell_env::ShellEnv;

    async fn git_with_env() -> Git {
        let mut env = ShellEnv::new();
        env.load_or_inherit().await.unwrap();
        Git::new(Arc::new(RwLock::new(env)))
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

    #[test]
    fn parses_shortstat() {
        assert_eq!(
            parse_shortstat(" 3 files changed, 10 insertions(+), 2 deletions(-)\n"),
            ShortStat { files: 3, added: 10, deleted: 2 }
        );
        assert_eq!(
            parse_shortstat(" 1 file changed, 1 deletion(-)"),
            ShortStat { files: 1, added: 0, deleted: 1 }
        );
        assert_eq!(parse_shortstat(""), ShortStat::default());
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
        assert!(!git
            .list_local_branches(&clone)
            .await
            .unwrap()
            .contains(&"feat/remote-only".to_string()));

        git.switch(&clone, "feat/remote-only").await.unwrap();
        assert_eq!(git.current_branch(&clone).await.unwrap(), "feat/remote-only");
    }

    #[tokio::test]
    async fn pulls_the_default_branch_checked_out_or_not() {
        let git = git_with_env().await;
        let (_origin_dir, origin) = init_repo();
        let clone_dir = tempfile::tempdir().unwrap();
        let clone = clone_dir.path().join("clone");
        run_ok(
            clone_dir.path(),
            &["git", "clone", "-q", &origin.to_string_lossy(), "clone"],
        );

        std::fs::write(origin.join("README.md"), "one\n").unwrap();
        run_ok(&origin, &["git", "commit", "-qam", "one"]);
        assert_eq!(git.pull_branch(&clone, "main").await.unwrap(), 1);
        assert_eq!(git.pull_branch(&clone, "main").await.unwrap(), 0);

        run_ok(&clone, &["git", "switch", "-qc", "feat/elsewhere"]);
        std::fs::write(origin.join("README.md"), "two\n").unwrap();
        run_ok(&origin, &["git", "commit", "-qam", "two"]);
        std::fs::write(origin.join("README.md"), "three\n").unwrap();
        run_ok(&origin, &["git", "commit", "-qam", "three"]);
        assert_eq!(git.pull_branch(&clone, "main").await.unwrap(), 2);
        assert_eq!(git.current_branch(&clone).await.unwrap(), "feat/elsewhere");
    }

    #[tokio::test]
    async fn validates_and_lists_branches() {
        let git = git_with_env().await;
        let (_dir, repo) = init_repo();
        let top = git.show_toplevel(&repo).await.unwrap();
        assert_eq!(top.canonicalize().unwrap(), repo.canonicalize().unwrap());
        let branches = git.list_branches(&repo).await.unwrap();
        assert!(branches.iter().any(|branch| branch.name == "main"));
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
                .any(|(path, added, _)| path == "README.md" && *added > 0)
        );
        assert!(stats.iter().any(|(path, _, _)| path == "extra.txt"));

        let dirty = git.switch(&worktree, "main").await;
        assert!(dirty.is_err());

        git.worktree_remove(&repo, &worktree, true).await.unwrap();
        git.branch_delete(&repo, "feature").await.unwrap();
    }

    #[tokio::test]
    async fn serialises_calls_on_one_worktree() {
        let git = git_with_env().await;
        let (_dir, repo) = init_repo();
        let a = git.clone();
        let b = git.clone();
        let repo_a = repo.clone();
        let repo_b = repo.clone();
        let (left, right) = tokio::join!(
            async move { a.status_porcelain(&repo_a).await },
            async move { b.status_porcelain(&repo_b).await }
        );
        left.unwrap();
        right.unwrap();
    }
}
