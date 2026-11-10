//! The workspace repository: each live workspace's record in memory (its row, plus the setup
//! activity only the running app knows) over its row in SQLite. A setter for a field the record
//! holds writes both; the PR, summary and run command live only in the row. The lifecycle has
//! this one owner: a thread's status reaches the workspace row inside `Store::set_thread_status`
//! (the record follows the row), a stopped teardown hands the workspace back through
//! [`Workspaces::hand_back`], and a quit is put right by [`restore_after_quit`], all by
//! [`once_stopped`] and `status_from_threads`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::sync::RwLock;

use crate::error::{Error, Result};
use crate::harness_config::HarnessEnv;
use crate::model::{ThreadStatus, WorkspaceKind, WorkspaceLifecycle, status_from_threads};
use crate::paths;
use crate::store::Store;
use crate::store::types::{RepoRecord, ThreadPicks, ThreadRow, WorkspaceRow};

/// A workspace as the webview shows it: its row, plus what only the running app knows.
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
    pub kind: WorkspaceKind,
    pub activity: WorkspaceActivity,
}

/// What a workspace is doing that only the running app knows: how far its setup got. The webview
/// words it, by the workspace's kind; what a review came to is the stored `review_state`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum WorkspaceActivity {
    /// Nothing beyond its lifecycle.
    #[default]
    None,
    /// Making the worktree and running the repo's setup (a review: checking out the PR).
    SettingUp,
    /// Setup is done and the agent is launching with its first prompt.
    StartingAgent,
    /// The agent has its first prompt.
    AgentStarted,
    /// `command` and its exit code when a setup command failed, else setup failed before one ran.
    SetupFailed {
        command: Option<String>,
        #[serde(rename = "exitCode")]
        exit_code: Option<i32>,
    },
}

impl WorkspaceRecord {
    fn from_row(row: &WorkspaceRow, repo: &RepoRecord) -> Self {
        Self {
            id: row.id.clone(),
            repo_id: row.repo_id.clone(),
            repo_path: paths::expand_tilde(&repo.path)
                .to_string_lossy()
                .to_string(),
            name: row.name.clone(),
            branch: row.branch.clone(),
            base: row.base.clone(),
            worktree_path: row.worktree_path.clone(),
            status: row.status,
            kind: row.kind,
            activity: WorkspaceActivity::None,
        }
    }

    pub fn worktree(&self) -> PathBuf {
        PathBuf::from(&self.worktree_path)
    }

    /// What a command run for the workspace (setup, the app) is told about it.
    pub fn harness_env(&self) -> HarnessEnv {
        HarnessEnv {
            repo_path: self.repo_path.clone(),
            worktree_path: self.worktree_path.clone(),
            workspace: self.name.clone(),
            branch: self.branch.clone(),
            base: self.base.clone(),
        }
    }
}

#[derive(Default)]
struct Inner {
    records: HashMap<String, WorkspaceRecord>,
}

#[derive(Clone)]
pub struct Workspaces {
    store: Store,
    inner: Arc<RwLock<Inner>>,
}

impl Workspaces {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            inner: Arc::new(RwLock::new(Inner::default())),
        }
    }

    /// A live workspace, read from the store the first time it's asked for.
    pub async fn get(&self, id: &str) -> Result<WorkspaceRecord> {
        if let Some(record) = self.inner.read().await.records.get(id) {
            return Ok(record.clone());
        }
        let mut inner = self.inner.write().await;
        load(&self.store, &mut inner, id).cloned()
    }

    /// Every live workspace, in the order they were created.
    pub async fn live(&self) -> Result<Vec<WorkspaceRecord>> {
        let rows = self.store.live_workspaces()?;
        let repos = self.store.repos()?;
        Ok(self.records_for(&rows, &repos).await)
    }

    /// Archived rows, and rows whose repo was removed, are left out.
    pub async fn records_for(
        &self,
        rows: &[WorkspaceRow],
        repos: &[RepoRecord],
    ) -> Vec<WorkspaceRecord> {
        let mut inner = self.inner.write().await;
        let mut records = Vec::with_capacity(rows.len());
        for row in rows.iter().filter(|row| row.archived_at.is_none()) {
            if let Some(record) = inner.records.get(&row.id) {
                records.push(record.clone());
                continue;
            }
            let Some(repo) = repos.iter().find(|repo| repo.id == row.repo_id) else {
                log::warn!("workspace {} belongs to a removed repo", row.id);
                continue;
            };
            let record = WorkspaceRecord::from_row(row, repo);
            inner.records.insert(row.id.clone(), record.clone());
            records.push(record);
        }
        records
    }

    /// The worktree folder must not be in use: `feat/x` and `feat-x` slug to the same folder, and
    /// two workspaces sharing a checkout would lose each other's work on teardown.
    pub async fn register(
        &self,
        row: WorkspaceRow,
        thread: &ThreadRow,
        picks: &ThreadPicks,
        setup_message: &str,
        repo: &RepoRecord,
    ) -> Result<WorkspaceRecord> {
        let mut inner = self.inner.write().await;
        let worktree = PathBuf::from(&row.worktree_path);
        ensure_worktree_free(&inner.records, &row.id, &worktree)?;
        if let Some(parent) = worktree.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                Error::Workspace(format!("couldn't create {}: {error}", parent.display()))
            })?;
        }
        self.store
            .insert_workspace_with_thread(&row, thread, picks, setup_message)?;
        let record = WorkspaceRecord::from_row(&row, repo);
        inner.records.insert(row.id.clone(), record.clone());
        Ok(record)
    }

    pub async fn set_status(
        &self,
        id: &str,
        status: WorkspaceLifecycle,
    ) -> Result<WorkspaceRecord> {
        self.update(id, |store, record| {
            assert_transition(record.status, status)?;
            store.set_workspace_status(id, status)?;
            record.status = status;
            Ok(())
        })
        .await
    }

    /// Starts tearing the workspace down, checked and set under one lock so two teardowns can't
    /// both start. Returns the workspace as it was before, to hand back if the teardown fails.
    pub async fn begin_teardown(&self, id: &str) -> Result<WorkspaceRecord> {
        let mut before = None;
        self.update(id, |store, record| {
            if record.status == WorkspaceLifecycle::TearingDown {
                return Err(Error::Workspace(format!(
                    "workspace {id} is already being torn down"
                )));
            }
            assert_transition(record.status, WorkspaceLifecycle::TearingDown)?;
            before = Some(record.clone());
            store.set_workspace_status(id, WorkspaceLifecycle::TearingDown)?;
            record.status = WorkspaceLifecycle::TearingDown;
            Ok(())
        })
        .await?;
        before.ok_or_else(|| Error::unknown_workspace(id))
    }

    /// Takes a failed setup back to provisioning. Anything else is refused, so a second Retry
    /// or Skip can't start a run over one already going.
    pub async fn restart_provisioning(&self, id: &str) -> Result<WorkspaceRecord> {
        self.update(id, |store, record| {
            if record.status != WorkspaceLifecycle::ProvisioningFailed {
                return Err(Error::Workspace(
                    "Setup isn't failed, so there's nothing to retry or skip".into(),
                ));
            }
            store.set_workspace_status(id, WorkspaceLifecycle::Provisioning)?;
            record.status = WorkspaceLifecycle::Provisioning;
            record.activity = WorkspaceActivity::SettingUp;
            Ok(())
        })
        .await
    }

    /// Fails setup, if the workspace is still setting up: one being torn down stays so. Returns
    /// whether it failed.
    pub async fn fail_provisioning(&self, id: &str, activity: WorkspaceActivity) -> Result<bool> {
        let mut failed = false;
        self.update(id, |store, record| {
            if record.status != WorkspaceLifecycle::Provisioning {
                return Ok(());
            }
            store.set_workspace_status(id, WorkspaceLifecycle::ProvisioningFailed)?;
            record.status = WorkspaceLifecycle::ProvisioningFailed;
            record.activity = activity;
            failed = true;
            Ok(())
        })
        .await?;
        Ok(failed)
    }

    /// Ends setup: the thread's status, then the workspace's. In that order, the thread
    /// moving can't re-derive the workspace's status, which setup still holds, so `lifecycle`
    /// is what it ends as. Refused unless the workspace is still setting up, so setup can't
    /// bring back one being torn down.
    pub async fn finish_provisioning(
        &self,
        id: &str,
        thread_id: &str,
        thread: ThreadStatus,
        lifecycle: WorkspaceLifecycle,
        activity: WorkspaceActivity,
    ) -> Result<()> {
        let still_setting_up =
            || Error::Workspace(format!("workspace {id} is no longer being set up"));
        if self.get(id).await?.status != WorkspaceLifecycle::Provisioning {
            return Err(still_setting_up());
        }
        self.set_thread_status(thread_id, id, thread).await?;
        self.update(id, |store, record| {
            if record.status != WorkspaceLifecycle::Provisioning {
                return Err(still_setting_up());
            }
            assert_transition(record.status, lifecycle)?;
            store.set_workspace_status(id, lifecycle)?;
            record.status = lifecycle;
            record.activity = activity;
            Ok(())
        })
        .await?;
        Ok(())
    }

    pub async fn rename(&self, id: &str, name: &str) -> Result<WorkspaceRecord> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::Workspace("Workspace name is required".into()));
        }
        self.update(id, |store, record| {
            store.set_workspace_name(id, name)?;
            record.name = name.to_string();
            Ok(())
        })
        .await
    }

    /// Renames the workspace unless it was renamed since it was called `from`. Returns whether
    /// it was.
    pub async fn rename_from(&self, id: &str, from: &str, to: &str) -> Result<bool> {
        let mut renamed = false;
        self.update(id, |store, record| {
            if record.name == from {
                store.set_workspace_name(id, to)?;
                record.name = to.to_string();
                renamed = true;
            }
            Ok(())
        })
        .await?;
        Ok(renamed)
    }

    /// Moves a workspace onto another branch before its worktree exists, keeping the folder
    /// named after the branch.
    pub async fn retarget(&self, id: &str, branch: &str) -> Result<WorkspaceRecord> {
        let mut inner = self.inner.write().await;
        let current = load(&self.store, &mut inner, id)?.clone();
        let worktree = Path::new(&current.worktree_path).with_file_name(paths::branch_slug(branch));
        if worktree != Path::new(&current.worktree_path) {
            ensure_worktree_free(&inner.records, id, &worktree)?;
        }
        let worktree = worktree.to_string_lossy().to_string();
        self.store.set_workspace_checkout(id, branch, &worktree)?;
        let record = load(&self.store, &mut inner, id)?;
        record.branch = branch.to_string();
        record.worktree_path = worktree;
        Ok(record.clone())
    }

    /// The branch now checked out in the workspace's worktree.
    pub async fn set_branch(&self, id: &str, branch: &str) -> Result<WorkspaceRecord> {
        self.update(id, |store, record| {
            store.set_workspace_branch(id, branch)?;
            record.branch = branch.to_string();
            Ok(())
        })
        .await
    }

    /// The branch the workspace is compared with, when its pull request was retargeted.
    pub async fn set_base(&self, id: &str, base: &str) -> Result<WorkspaceRecord> {
        self.update(id, |store, record| {
            store.set_workspace_base(id, base)?;
            record.base = base.to_string();
            Ok(())
        })
        .await
    }

    /// The number of the pull request opened for the live workspace, if one was.
    pub async fn pr_number(&self, id: &str) -> Result<Option<i64>> {
        self.get(id).await?;
        Ok(self
            .store
            .workspace_by_id(id)?
            .and_then(|row| row.pr_number))
    }

    pub fn set_pr(&self, id: &str, number: Option<i64>, html_url: Option<&str>) -> Result<()> {
        self.store.set_workspace_pr(id, number, html_url)
    }

    /// The text the workspace's Lead shows above its thread, written by Cormux.
    pub fn set_summary(&self, id: &str, summary: &str) -> Result<()> {
        self.store.set_workspace_summary(id, summary)
    }

    /// The workspace's own run command; `None` goes back to the repo's.
    pub fn set_run_command(&self, id: &str, command: Option<&str>) -> Result<()> {
        self.store.set_workspace_run_command(id, command)
    }

    pub async fn set_activity(
        &self,
        id: &str,
        activity: WorkspaceActivity,
    ) -> Result<WorkspaceRecord> {
        self.update(id, |_, record| {
            record.activity = activity;
            Ok(())
        })
        .await
    }

    /// Saves a thread's status and the workspace status that follows from it (see
    /// `status_from_threads`). A scratch's thread has no workspace, so only the thread moves.
    pub async fn set_thread_status(
        &self,
        thread_id: &str,
        workspace_id: &str,
        status: ThreadStatus,
    ) -> Result<()> {
        let mut inner = self.inner.write().await;
        let derived = self.store.set_thread_status(thread_id, status)?;
        if let (Some(record), Some(derived)) = (inner.records.get_mut(workspace_id), derived)
            && derived != record.status
        {
            assert_transition(record.status, derived)?;
            record.status = derived;
        }
        Ok(())
    }

    /// A teardown that stopped the workspace's agents and setup gave up: the workspace goes back
    /// to what it is with them stopped (see [`once_stopped`]).
    pub async fn hand_back(&self, id: &str, before: WorkspaceLifecycle) -> Result<WorkspaceRecord> {
        let threads = self.thread_statuses(id)?;
        self.set_status(id, once_stopped(before, &threads)).await
    }

    pub async fn can_switch_branch(&self, id: &str) -> Result<()> {
        match self.branch_lock(id).await? {
            Some(reason) => Err(Error::Workspace(reason.into())),
            None => Ok(()),
        }
    }

    /// Why the workspace's branch can't move now; see [`branch_lock`].
    pub async fn branch_lock(&self, id: &str) -> Result<Option<&'static str>> {
        let record = self.get(id).await?;
        Ok(branch_lock(record.status, self.thread_statuses(id)?))
    }

    pub async fn archive(&self, id: &str) -> Result<()> {
        let mut inner = self.inner.write().await;
        self.store.archive_workspace(id)?;
        inner.records.remove(id);
        Ok(())
    }

    fn thread_statuses(&self, id: &str) -> Result<Vec<ThreadStatus>> {
        Ok(self
            .store
            .threads_for_workspace(id)?
            .into_iter()
            .map(|thread| thread.status)
            .collect())
    }

    async fn update(
        &self,
        id: &str,
        change: impl FnOnce(&Store, &mut WorkspaceRecord) -> Result<()>,
    ) -> Result<WorkspaceRecord> {
        let mut inner = self.inner.write().await;
        let record = load(&self.store, &mut inner, id)?;
        change(&self.store, record)?;
        Ok(record.clone())
    }
}

/// Why a workspace's branch can't move now: setup is still making the worktree, or an agent
/// is working in it. `None` when it can.
pub fn branch_lock(
    status: WorkspaceLifecycle,
    threads: impl IntoIterator<Item = ThreadStatus>,
) -> Option<&'static str> {
    if status == WorkspaceLifecycle::Provisioning {
        Some("Wait for setup to finish to change branches.")
    } else if threads.into_iter().any(ThreadStatus::is_working) {
        Some("Pause or stop the running agents to change branches.")
    } else {
        None
    }
}

/// A workspace whose branch can't move now, and why, for the snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceBranchLock {
    pub workspace_id: String,
    pub reason: String,
}

pub fn branch_locks(
    workspaces: &[WorkspaceRecord],
    threads: &[crate::store::types::ThreadRow],
) -> Vec<WorkspaceBranchLock> {
    workspaces
        .iter()
        .filter_map(|workspace| {
            let statuses = threads
                .iter()
                .filter(|thread| thread.workspace_id == workspace.id)
                .map(|thread| thread.status);
            branch_lock(workspace.status, statuses).map(|reason| WorkspaceBranchLock {
                workspace_id: workspace.id.clone(),
                reason: reason.into(),
            })
        })
        .collect()
}

/// Whether a workspace needs the user: nothing works in it, so it needs prompting or closing, or
/// an agent waits on one of its `pending` approvals. `own` is its own thread's status. The
/// snapshot carries it for the sidebar, switcher and palette. An agent asking about threads gets
/// orchestration's rule instead, where an idle thread is done rather than waiting.
pub fn needs_attention(
    workspace: &WorkspaceRecord,
    own: Option<ThreadStatus>,
    pending: usize,
) -> bool {
    let working = own.is_some_and(ThreadStatus::is_working)
        || matches!(
            workspace.status,
            WorkspaceLifecycle::Provisioning | WorkspaceLifecycle::Running
        );
    !working || pending > 0
}

fn load<'a>(store: &Store, inner: &'a mut Inner, id: &str) -> Result<&'a mut WorkspaceRecord> {
    if !inner.records.contains_key(id) {
        let row = store
            .workspace_by_id(id)?
            .ok_or_else(|| Error::unknown_workspace(id))?;
        if row.archived_at.is_some() {
            return Err(Error::Workspace(format!("workspace {id} is archived")));
        }
        let repo = store.repo(&row.repo_id)?;
        inner
            .records
            .insert(id.to_string(), WorkspaceRecord::from_row(&row, &repo));
    }
    inner
        .records
        .get_mut(id)
        .ok_or_else(|| Error::unknown_workspace(id))
}

/// What a workspace is once everything that was running in it has stopped: setup that was
/// running has failed, a workspace whose agents were mid-turn is idle, and one whose teardown
/// stopped follows its threads, or failed setup if setup was what it stopped. `threads` are its
/// live threads' statuses, with none mid-turn any more.
fn once_stopped(before: WorkspaceLifecycle, threads: &[ThreadStatus]) -> WorkspaceLifecycle {
    use WorkspaceLifecycle::{
        Idle, Paused, Provisioning, ProvisioningFailed, Ready, Running, TearingDown,
    };
    match before {
        Provisioning | ProvisioningFailed => ProvisioningFailed,
        Running | Paused => Idle,
        Ready | Idle => before,
        TearingDown if threads.contains(&ThreadStatus::Provisioning) => ProvisioningFailed,
        TearingDown => status_from_threads(Idle, threads.iter().copied()),
    }
}

/// No engine, setup or teardown survives the app closing. A turn in flight then is over, so its
/// thread is idle, and each workspace is what it is with everything stopped (see
/// [`once_stopped`]). A teardown isn't finished unasked: that would delete the worktree and
/// maybe the branch without the user seeing what's in them now, so they tear it down again.
pub fn restore_after_quit(store: &Store) -> Result<()> {
    store.with_tx(|conn| {
        for status in ThreadStatus::MID_TURN {
            conn.execute(
                "UPDATE threads SET status = ?1 WHERE status = ?2",
                [ThreadStatus::Idle.as_str(), status.as_str()],
            )?;
        }
        let workspaces: Vec<(String, String)> = conn
            .prepare("SELECT id, status FROM workspaces WHERE archived_at IS NULL")?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        for (id, raw) in workspaces {
            let before = WorkspaceLifecycle::parse(&raw);
            let after = once_stopped(before, &live_thread_statuses(conn, &id)?);
            if after != before {
                assert_transition(before, after)?;
                write_status(conn, &id, after)?;
            }
        }
        Ok(())
    })
}

fn live_thread_statuses(
    conn: &rusqlite::Connection,
    workspace_id: &str,
) -> Result<Vec<ThreadStatus>> {
    Ok(conn
        .prepare_cached("SELECT status FROM threads WHERE workspace_id = ?1 AND retired = 0")?
        .query_map([workspace_id], |row| row.get::<_, String>(0))?
        .map(|raw| raw.map(|raw| ThreadStatus::parse(&raw)))
        .collect::<rusqlite::Result<_>>()?)
}

fn write_status(
    conn: &rusqlite::Connection,
    workspace_id: &str,
    status: WorkspaceLifecycle,
) -> Result<()> {
    conn.prepare_cached("UPDATE workspaces SET status = ?1 WHERE id = ?2")?
        .execute(rusqlite::params![status.as_str(), workspace_id])?;
    Ok(())
}

fn ensure_worktree_free(
    records: &HashMap<String, WorkspaceRecord>,
    workspace_id: &str,
    worktree: &Path,
) -> Result<()> {
    let taken = records
        .values()
        .any(|other| other.id != workspace_id && Path::new(&other.worktree_path) == worktree);
    if taken || worktree.exists() {
        return Err(Error::Workspace(format!(
            "worktree folder already in use: {}",
            worktree.display()
        )));
    }
    Ok(())
}

/// The lifecycle's legal moves. A failed teardown hands the workspace back as it can now be:
/// its threads were stopped, and so was any setup.
fn assert_transition(from: WorkspaceLifecycle, to: WorkspaceLifecycle) -> Result<()> {
    use WorkspaceLifecycle::{
        Idle, Paused, Provisioning, ProvisioningFailed, Ready, Running, TearingDown,
    };
    if from == to {
        return Ok(());
    }
    let ok = matches!(
        (from, to),
        (
            Provisioning,
            Ready | Running | ProvisioningFailed | TearingDown
        ) | (Ready, Running | Paused | Idle | TearingDown)
            | (
                Running | Paused | Idle,
                Ready | Running | Paused | Idle | TearingDown
            )
            | (ProvisioningFailed, Provisioning | TearingDown)
            | (TearingDown, Ready | Idle | ProvisioningFailed)
    );
    if ok {
        Ok(())
    } else {
        Err(Error::Workspace(format!(
            "illegal transition {from:?} → {to:?}"
        )))
    }
}

/// The workspace row's columns that change after it's made; only `Workspaces` writes them.
impl Store {
    pub fn set_workspace_status(
        &self,
        workspace_id: &str,
        status: WorkspaceLifecycle,
    ) -> Result<()> {
        self.with_conn(|conn| write_status(conn, workspace_id, status))
    }

    /// Writes the status and re-derives the workspace's from its threads (see
    /// `status_from_threads`), so the two can't disagree, and returns the workspace's. A thread
    /// with no workspace, or whose workspace is archived, moves alone.
    pub fn set_thread_status(
        &self,
        thread_id: &str,
        status: ThreadStatus,
    ) -> Result<Option<WorkspaceLifecycle>> {
        self.with_tx(|conn| {
            conn.prepare_cached("UPDATE threads SET status = ?1 WHERE id = ?2")?
                .execute(rusqlite::params![status.as_str(), thread_id])?;
            let workspace = conn
                .prepare_cached(
                    "SELECT w.id, w.status FROM workspaces w
                     JOIN threads t ON t.workspace_id = w.id
                     WHERE t.id = ?1 AND w.archived_at IS NULL",
                )?
                .query_row([thread_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .optional()?;
            let Some((workspace_id, current)) = workspace else {
                return Ok(None);
            };
            let current = WorkspaceLifecycle::parse(&current);
            let derived = status_from_threads(
                current,
                live_thread_statuses(conn, &workspace_id)?.into_iter(),
            );
            if derived != current {
                write_status(conn, &workspace_id, derived)?;
            }
            Ok(Some(derived))
        })
    }

    pub fn set_workspace_name(&self, workspace_id: &str, name: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE workspaces SET name = ?1 WHERE id = ?2",
                rusqlite::params![name, workspace_id],
            )?;
            Ok(())
        })
    }

    pub fn set_workspace_branch(&self, workspace_id: &str, branch: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE workspaces SET branch = ?1 WHERE id = ?2",
                rusqlite::params![branch, workspace_id],
            )?;
            Ok(())
        })
    }

    pub fn set_workspace_base(&self, workspace_id: &str, base: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE workspaces SET base = ?1 WHERE id = ?2",
                rusqlite::params![base, workspace_id],
            )?;
            Ok(())
        })
    }

    pub fn set_workspace_checkout(
        &self,
        workspace_id: &str,
        branch: &str,
        worktree_path: &str,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE workspaces SET branch = ?1, worktree_path = ?2 WHERE id = ?3",
                rusqlite::params![branch, worktree_path, workspace_id],
            )?;
            Ok(())
        })
    }

    pub fn set_workspace_pr(
        &self,
        workspace_id: &str,
        pr_number: Option<i64>,
        pr_html_url: Option<&str>,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE workspaces SET pr_number = ?1, pr_html_url = ?2 WHERE id = ?3",
                rusqlite::params![pr_number, pr_html_url, workspace_id],
            )?;
            Ok(())
        })
    }

    pub fn set_workspace_run_command(
        &self,
        workspace_id: &str,
        command: Option<&str>,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE workspaces SET run_command = ?1 WHERE id = ?2",
                rusqlite::params![command, workspace_id],
            )?;
            Ok(())
        })
    }

    pub fn set_workspace_summary(&self, workspace_id: &str, summary: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE workspaces SET summary = ?1, summary_at = datetime('now') WHERE id = ?2",
                rusqlite::params![summary, workspace_id],
            )?;
            Ok(())
        })
    }

    pub fn archive_workspace(&self, workspace_id: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE workspaces SET archived_at = datetime('now'), worktree_path = ''
                 WHERE id = ?1",
                rusqlite::params![workspace_id],
            )?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ThreadRole;

    #[test]
    fn a_workspace_hands_back_what_was_stopped_in_it() {
        use ThreadStatus::{Idle, Provisioning};
        use WorkspaceLifecycle as L;
        let cases = [
            (L::Provisioning, vec![Provisioning], L::ProvisioningFailed),
            (
                L::ProvisioningFailed,
                vec![Provisioning],
                L::ProvisioningFailed,
            ),
            (L::Running, vec![Idle], L::Idle),
            (L::Paused, vec![Idle], L::Idle),
            (L::Ready, vec![Idle], L::Ready),
            (L::Idle, vec![Idle], L::Idle),
            (
                L::TearingDown,
                vec![Idle, Provisioning],
                L::ProvisioningFailed,
            ),
            (L::TearingDown, vec![Idle], L::Idle),
            (L::TearingDown, vec![], L::Ready),
        ];
        for (before, threads, after) in cases {
            assert_eq!(
                once_stopped(before, &threads),
                after,
                "{before:?} {threads:?}"
            );
        }
    }

    #[test]
    fn whatever_was_stopped_the_workspace_moves_legally() {
        use WorkspaceLifecycle as L;
        let stopped = [ThreadStatus::Idle, ThreadStatus::Provisioning];
        let thread_sets = [vec![], vec![stopped[0]], vec![stopped[1]], stopped.to_vec()];
        for before in [
            L::Provisioning,
            L::Ready,
            L::Running,
            L::Paused,
            L::Idle,
            L::TearingDown,
            L::ProvisioningFailed,
        ] {
            for threads in &thread_sets {
                assert_transition(before, once_stopped(before, threads)).unwrap();
            }
        }
    }

    fn store_with_repo(default_branch: &str) -> Store {
        let store = Store::in_memory().unwrap();
        store
            .upsert_repo(&RepoRecord {
                path: "/tmp/app".into(),
                name: "app".into(),
                default_branch: default_branch.into(),
                ..crate::store::fixtures::repo("r1")
            })
            .unwrap();
        store
    }

    fn row(id: &str, branch: &str, base: &str, root: &Path) -> WorkspaceRow {
        WorkspaceRow::new(
            WorkspaceKind::Lead,
            id,
            "r1",
            "Work",
            branch,
            base,
            root.join(paths::branch_slug(branch)).to_string_lossy(),
        )
    }

    async fn registered(store: &Store, root: &Path) -> Workspaces {
        let workspaces = Workspaces::new(store.clone());
        let repo = store.repo_by_id("r1").unwrap().unwrap();
        workspaces
            .register(
                row("w", "feat/draft", "release", root),
                &ThreadRow::new("t", "w", ThreadRole::Lead, "claude"),
                &Default::default(),
                "",
                &repo,
            )
            .await
            .unwrap();
        workspaces
    }

    #[tokio::test]
    async fn a_restart_reads_back_what_was_written_including_the_base() {
        let store = store_with_repo("main");
        let root = tempfile::tempdir().unwrap();
        let workspaces = registered(&store, root.path()).await;
        workspaces.rename("w", "Login").await.unwrap();
        workspaces.retarget("w", "fix/login").await.unwrap();
        workspaces
            .set_status("w", WorkspaceLifecycle::Ready)
            .await
            .unwrap();

        let restarted = Workspaces::new(store.clone());
        let record = restarted.get("w").await.unwrap();
        assert_eq!(record.name, "Login");
        assert_eq!(record.branch, "fix/login");
        assert_eq!(record.base, "release");
        assert!(record.worktree_path.ends_with("/fix-login"));
        assert_eq!(record.status, WorkspaceLifecycle::Ready);
        assert_eq!(record.repo_path, "/tmp/app");
        assert_eq!(
            store.own_thread("w").unwrap().unwrap().role,
            Some(ThreadRole::Lead)
        );
        assert_eq!(restarted.live().await.unwrap().len(), 1);
    }

    /// Bug: teardown checked the status and set it under separate locks, so two teardowns could
    /// both start.
    #[tokio::test]
    async fn only_one_of_two_teardowns_begins() {
        let store = store_with_repo("main");
        let root = tempfile::tempdir().unwrap();
        let workspaces = registered(&store, root.path()).await;
        workspaces
            .set_status("w", WorkspaceLifecycle::Ready)
            .await
            .unwrap();

        let (first, second) = tokio::join!(
            workspaces.begin_teardown("w"),
            workspaces.begin_teardown("w")
        );

        let begun: Vec<_> = [first, second].into_iter().filter_map(Result::ok).collect();
        assert_eq!(begun.len(), 1);
        assert_eq!(begun[0].status, WorkspaceLifecycle::Ready);
        assert_eq!(
            workspaces.get("w").await.unwrap().status,
            WorkspaceLifecycle::TearingDown
        );
    }

    #[tokio::test]
    async fn a_summary_and_pr_are_saved() {
        let store = store_with_repo("main");
        let root = tempfile::tempdir().unwrap();
        let workspaces = registered(&store, root.path()).await;
        workspaces.set_summary("w", "Started.").unwrap();
        workspaces
            .set_pr("w", Some(7), Some("https://github.com/o/r/pull/7"))
            .unwrap();
        let row = store.workspace_by_id("w").unwrap().unwrap();
        assert_eq!(row.summary.as_deref(), Some("Started."));
        assert_eq!(row.pr_number, Some(7));
    }

    #[tokio::test]
    async fn a_second_workspace_cant_take_the_same_folder() {
        let store = store_with_repo("main");
        let root = tempfile::tempdir().unwrap();
        let workspaces = registered(&store, root.path()).await;
        let repo = store.repo_by_id("r1").unwrap().unwrap();
        let error = workspaces
            .register(
                row("w2", "feat-draft", "main", root.path()),
                &ThreadRow::new("t2", "w2", ThreadRole::Lead, "claude"),
                &Default::default(),
                "",
                &repo,
            )
            .await
            .unwrap_err();
        assert!(error.to_string().contains("already in use"), "{error}");
        assert!(store.workspace_by_id("w2").unwrap().is_none());
    }

    #[tokio::test]
    async fn a_running_thread_blocks_a_branch_switch() {
        let store = store_with_repo("main");
        let root = tempfile::tempdir().unwrap();
        let workspaces = registered(&store, root.path()).await;
        assert!(workspaces.can_switch_branch("w").await.is_err());
        workspaces
            .set_status("w", WorkspaceLifecycle::Ready)
            .await
            .unwrap();

        workspaces
            .set_thread_status("t", "w", ThreadStatus::Running)
            .await
            .unwrap();
        assert!(workspaces.can_switch_branch("w").await.is_err());
        let fresh = Workspaces::new(store.clone());
        assert!(
            fresh.can_switch_branch("w").await.is_err(),
            "the stored status counts, not what this instance saw"
        );
        assert_eq!(
            workspaces.get("w").await.unwrap().status,
            WorkspaceLifecycle::Running
        );

        workspaces
            .set_thread_status("t", "w", ThreadStatus::Idle)
            .await
            .unwrap();
        workspaces.can_switch_branch("w").await.unwrap();
        assert_eq!(
            store.workspace_by_id("w").unwrap().unwrap().status,
            WorkspaceLifecycle::Idle
        );
    }

    #[tokio::test]
    async fn an_illegal_move_changes_neither_memory_nor_the_row() {
        let store = store_with_repo("main");
        let root = tempfile::tempdir().unwrap();
        let workspaces = registered(&store, root.path()).await;
        workspaces
            .set_status("w", WorkspaceLifecycle::TearingDown)
            .await
            .unwrap();
        assert!(
            workspaces
                .set_status("w", WorkspaceLifecycle::Running)
                .await
                .is_err()
        );
        assert_eq!(
            workspaces.get("w").await.unwrap().status,
            WorkspaceLifecycle::TearingDown
        );
        assert_eq!(
            store.workspace_by_id("w").unwrap().unwrap().status,
            WorkspaceLifecycle::TearingDown
        );
    }

    #[tokio::test]
    async fn a_workspace_whose_repo_is_gone_doesnt_hide_the_others() {
        let store = store_with_repo("main");
        let root = tempfile::tempdir().unwrap();
        let workspaces = registered(&store, root.path()).await;
        let mut orphan = row("orphan", "feat/y", "main", root.path());
        orphan.repo_id = "gone".into();
        let repos = store.repos().unwrap();
        let rows = vec![orphan, store.workspace_by_id("w").unwrap().unwrap()];

        let records = workspaces.records_for(&rows, &repos).await;

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "w");
    }

    #[tokio::test]
    async fn an_archived_or_unknown_workspace_is_not_found() {
        let store = store_with_repo("main");
        let root = tempfile::tempdir().unwrap();
        let workspaces = registered(&store, root.path()).await;
        workspaces.archive("w").await.unwrap();
        assert!(workspaces.get("w").await.is_err());
        assert!(workspaces.get("nope").await.is_err());
        assert!(workspaces.live().await.unwrap().is_empty());
    }

    /// Bug: the Stack tab blamed running agents for a lock that was setup's.
    #[test]
    fn a_branch_lock_says_whether_setup_or_agents_hold_it() {
        use ThreadStatus::{Idle, Paused, Running};
        use WorkspaceLifecycle as Lifecycle;
        let setup = branch_lock(Lifecycle::Provisioning, [Idle]).unwrap();
        assert!(setup.contains("setup"), "{setup}");
        let agents = branch_lock(Lifecycle::Running, [Idle, Running]).unwrap();
        assert!(agents.contains("running agents"), "{agents}");
        assert_eq!(branch_lock(Lifecycle::Paused, [Paused, Idle]), None);
        assert_eq!(branch_lock(Lifecycle::ProvisioningFailed, [Idle]), None);
    }

    /// The browser harness reads the same cases (`src/lib/dev/harness/attention.test.ts`). A
    /// workspace is working as soon as its own thread is, before its lifecycle catches up.
    #[test]
    fn a_workspace_needs_you_when_nothing_works_in_it_or_an_approval_waits() {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Case {
            lifecycle: WorkspaceLifecycle,
            own: Option<ThreadStatus>,
            pending: usize,
            needs_attention: bool,
        }
        let cases: Vec<Case> = serde_json::from_str(include_str!(
            "../../src/lib/ipc/rule-cases/needs-attention.json"
        ))
        .unwrap();
        let repo = crate::store::fixtures::repo("r1");
        for case in cases {
            let workspace = WorkspaceRecord::from_row(
                &row("w", "feat", "main", Path::new("/tmp")).with_status(case.lifecycle),
                &repo,
            );
            assert_eq!(
                needs_attention(&workspace, case.own, case.pending),
                case.needs_attention,
                "{:?} {:?} {}",
                case.lifecycle,
                case.own,
                case.pending
            );
        }
    }
}
