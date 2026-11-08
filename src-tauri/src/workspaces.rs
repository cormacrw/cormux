//! The workspace repository: each workspace's live record in memory and its row in SQLite. Every
//! change goes through a setter here that writes both, so nothing else keeps them in step.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::sync::RwLock;

use crate::error::{Error, Result};
use crate::model::{ThreadStatus, WorkspaceKind, WorkspaceLifecycle, status_from_threads};
use crate::paths;
use crate::process::HarnessEnv;
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
    pub activity: String,
    pub prov_step: u8,
    pub setup_failed_command: Option<String>,
    pub setup_failed_exit_code: Option<i32>,
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
            activity: String::new(),
            prov_step: 0,
            setup_failed_command: None,
            setup_failed_exit_code: None,
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

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RenameWorkspaceInput {
    pub workspace_id: String,
    pub name: String,
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

    /// The records for these live rows. A row whose repo was removed is left out.
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

    /// Records a new workspace and its own thread, in memory and in the store. Its worktree
    /// folder must not be in use: `feat/x` and `feat-x` slug to the same folder, and two
    /// workspaces sharing a checkout would lose each other's work on teardown.
    pub async fn register(
        &self,
        row: WorkspaceRow,
        thread: &ThreadRow,
        picks: &ThreadPicks,
        repo: &RepoRecord,
    ) -> Result<WorkspaceRecord> {
        let mut inner = self.inner.write().await;
        let worktree = PathBuf::from(&row.worktree_path);
        ensure_worktree_free(&inner.records, &row.id, &worktree)?;
        if let Some(parent) = worktree.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.store
            .insert_workspace_with_thread(&row, thread, picks)?;
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

    pub fn set_pr(&self, id: &str, number: Option<i64>, html_url: Option<&str>) -> Result<()> {
        self.store.set_workspace_pr(id, number, html_url)
    }

    /// The text the workspace's Lead shows above its thread, written by Cormux.
    pub fn set_summary(&self, id: &str, summary: &str) -> Result<()> {
        self.store.set_workspace_summary(id, summary)
    }

    /// The workspace's own run command; blank goes back to the repo's.
    pub fn set_run_command(&self, id: &str, command: &str) -> Result<()> {
        let command = command.trim();
        self.store
            .set_workspace_run_command(id, (!command.is_empty()).then_some(command))
    }

    /// What setup is doing, or why it failed. Only the running app knows this.
    pub async fn set_activity(
        &self,
        id: &str,
        activity: &str,
        prov_step: u8,
        failure: Option<(String, i32)>,
    ) -> Result<WorkspaceRecord> {
        self.update(id, |_, record| {
            record.activity = activity.to_string();
            record.prov_step = prov_step;
            (record.setup_failed_command, record.setup_failed_exit_code) = match failure {
                Some((command, code)) => (Some(command), Some(code)),
                None => (None, None),
            };
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
        self.store.set_thread_status(thread_id, status)?;
        let Some(record) = inner.records.get_mut(workspace_id) else {
            return Ok(());
        };
        let derived = match self.store.workspace_by_id(workspace_id)? {
            Some(row) => row.status,
            None => status_from_threads(record.status, std::iter::once(status)),
        };
        if derived != record.status {
            assert_transition(record.status, derived)?;
            record.status = derived;
        }
        Ok(())
    }

    /// Branches can't move under a workspace that is being set up or has a thread at work.
    pub async fn can_switch_branch(&self, id: &str) -> Result<()> {
        let record = self.get(id).await?;
        if record.status == WorkspaceLifecycle::Provisioning {
            return Err(Error::Workspace(
                "branch switch refused while provisioning".into(),
            ));
        }
        let blocked = self.store.threads_for_workspace(id)?.iter().any(|thread| {
            matches!(
                thread.status,
                ThreadStatus::Running | ThreadStatus::Provisioning
            )
        });
        if blocked {
            return Err(Error::Workspace(
                "branch switch refused while a thread is running".into(),
            ));
        }
        Ok(())
    }

    /// The workspace is gone: its row is archived and its record dropped.
    pub async fn archive(&self, id: &str) -> Result<()> {
        let mut inner = self.inner.write().await;
        self.store.archive_workspace(id)?;
        inner.records.remove(id);
        Ok(())
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

/// The record in memory, read from the store into memory if it isn't there yet.
fn load<'a>(store: &Store, inner: &'a mut Inner, id: &str) -> Result<&'a mut WorkspaceRecord> {
    if !inner.records.contains_key(id) {
        let row = store
            .workspace_by_id(id)?
            .ok_or_else(|| Error::unknown_workspace(id))?;
        if row.archived_at.is_some() {
            return Err(Error::Workspace(format!("workspace {id} is archived")));
        }
        let repo = store
            .repo_by_id(&row.repo_id)?
            .ok_or_else(|| Error::unknown_repo(&row.repo_id))?;
        inner
            .records
            .insert(id.to_string(), WorkspaceRecord::from_row(&row, &repo));
    }
    inner
        .records
        .get_mut(id)
        .ok_or_else(|| Error::unknown_workspace(id))
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
    use WorkspaceLifecycle::{Idle, Provisioning, ProvisioningFailed, Ready, Running, TearingDown};
    if from == to {
        return Ok(());
    }
    let ok = matches!(
        (from, to),
        (
            Provisioning,
            Ready | Running | ProvisioningFailed | TearingDown
        ) | (Ready, Running | Idle | TearingDown)
            | (Running | Idle, Ready | Running | Idle | TearingDown)
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

#[tauri::command]
#[specta::specta]
pub async fn rename_workspace(
    state: tauri::State<'_, crate::state::AppState>,
    input: RenameWorkspaceInput,
) -> Result<()> {
    state
        .workspaces
        .rename(&input.workspace_id, &input.name)
        .await?;
    state.out.changed();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ThreadRole;

    fn store_with_repo(default_branch: &str) -> Store {
        let store = Store::in_memory().unwrap();
        store
            .upsert_repo(&RepoRecord {
                path: "/tmp/app".into(),
                name: "app".into(),
                default_branch: Some(default_branch.into()),
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
}
