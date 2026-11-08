use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{Error, Result};
use crate::model::{ThreadRole, ThreadStatus, WorkspaceKind, WorkspaceLifecycle};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepoRecord {
    pub id: String,
    pub path: String,
    pub name: String,
    pub default_branch: Option<String>,
    pub setup_commands: String,
    pub run_command: Option<String>,
    /// Running the app in one workspace stops it in the repo's other workspaces.
    pub single_instance: bool,
}

impl RepoRecord {
    pub fn default_branch_or_main(&self) -> &str {
        self.default_branch.as_deref().unwrap_or("main")
    }

    /// The default branch stays in the repo checkout, where the sidebar's refresh pulls
    /// it. Git lets a branch be checked out in one worktree only, so no workspace takes it.
    pub fn ensure_not_default_branch(&self, branch: &str) -> Result<()> {
        let default = self.default_branch_or_main();
        if branch.trim() == default {
            return Err(Error::Workspace(format!(
                "Workspaces can't check out {default}, the default branch of {}. Use another branch.",
                self.name
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRow {
    pub id: String,
    pub repo_id: String,
    pub name: String,
    pub branch: String,
    /// The branch the workspace is compared with and pulls from.
    pub base: String,
    pub worktree_path: String,
    pub status: WorkspaceLifecycle,
    pub created_at: String,
    /// What the workspace's Lead shows above its thread, written by Cormux when setup is done.
    pub summary: Option<String>,
    pub summary_at: Option<String>,
    pub kind: WorkspaceKind,
    pub pr_number: Option<i64>,
    pub pr_html_url: Option<String>,
    pub archived_at: Option<String>,
    /// The workspace's own run command, which Run starts instead of the repo's.
    pub run_command: Option<String>,
}

impl WorkspaceRow {
    /// A workspace about to be provisioned.
    pub fn new(
        kind: WorkspaceKind,
        id: impl Into<String>,
        repo_id: impl Into<String>,
        name: impl Into<String>,
        branch: impl Into<String>,
        base: impl Into<String>,
        worktree_path: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            repo_id: repo_id.into(),
            name: name.into(),
            branch: branch.into(),
            base: base.into(),
            worktree_path: worktree_path.into(),
            status: WorkspaceLifecycle::Provisioning,
            created_at: String::new(),
            summary: None,
            summary_at: None,
            kind,
            pr_number: None,
            pr_html_url: None,
            archived_at: None,
            run_command: None,
        }
    }

    #[cfg(test)]
    pub fn with_status(self, status: WorkspaceLifecycle) -> Self {
        Self { status, ..self }
    }

    pub fn with_pr(self, number: i64, html_url: Option<String>) -> Self {
        Self {
            pr_number: Some(number),
            pr_html_url: html_url,
            ..self
        }
    }
}

/// The model and effort a new thread starts on; `None` leaves the engine's default.
#[derive(Debug, Clone, Default)]
pub struct ThreadPicks {
    pub model: Option<String>,
    pub effort: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ThreadRow {
    pub id: String,
    pub workspace_id: String,
    /// Display text. What the thread does is its `role`.
    pub title: String,
    /// The workspace's own Lead or Reviewer; none for any other thread.
    pub role: Option<ThreadRole>,
    pub engine: String,
    pub session_id: Option<String>,
    pub status: ThreadStatus,
    pub used_tokens: Option<i64>,
    pub context_size: Option<i64>,
    pub cost_usd: Option<f64>,
    /// Kept to read but can't be continued: a workspace's older thread, or a scratch's.
    pub retired: bool,
    /// The thread that started this one through `create_thread`.
    pub parent_thread_id: Option<String>,
}

impl ThreadRow {
    /// A workspace's own thread, waiting for its workspace to be provisioned.
    pub fn new(
        id: impl Into<String>,
        workspace_id: impl Into<String>,
        role: ThreadRole,
        engine: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            workspace_id: workspace_id.into(),
            title: role.title().into(),
            role: Some(role),
            engine: engine.into(),
            session_id: None,
            status: ThreadStatus::Provisioning,
            used_tokens: None,
            context_size: None,
            cost_usd: None,
            retired: false,
            parent_thread_id: None,
        }
    }

    #[cfg(test)]
    pub fn with_status(self, status: ThreadStatus) -> Self {
        Self { status, ..self }
    }
}
