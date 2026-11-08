use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{Error, Result};
use crate::model::{
    SUMMARY_MODEL_LABEL, ThreadRole, ThreadStatus, WorkspaceKind, WorkspaceLifecycle,
};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SettingRow {
    pub key: String,
    pub value: String,
}

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
    pub status: String,
    pub created_at: String,
    pub summary: Option<String>,
    pub summary_at: Option<String>,
    pub summary_source: String,
    pub kind: WorkspaceKind,
    pub pr_number: Option<i64>,
    pub pr_html_url: Option<String>,
    pub modified_files: i64,
    pub archived_at: Option<String>,
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
            status: WorkspaceLifecycle::Provisioning.as_str().into(),
            created_at: String::new(),
            summary: None,
            summary_at: None,
            summary_source: SUMMARY_MODEL_LABEL.into(),
            kind,
            pr_number: None,
            pr_html_url: None,
            modified_files: 0,
            archived_at: None,
        }
    }

    #[cfg(test)]
    pub fn with_status(self, status: WorkspaceLifecycle) -> Self {
        Self {
            status: status.as_str().into(),
            ..self
        }
    }

    pub fn with_pr(self, number: i64, html_url: Option<String>) -> Self {
        Self {
            pr_number: Some(number),
            pr_html_url: html_url,
            ..self
        }
    }

    pub fn lifecycle(&self) -> WorkspaceLifecycle {
        WorkspaceLifecycle::parse(&self.status)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
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
    pub status: String,
    pub used_tokens: Option<i64>,
    pub context_size: Option<i64>,
    pub cost_usd: Option<f64>,
    pub transcript_readonly: bool,
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
            status: ThreadStatus::Provisioning.into(),
            used_tokens: None,
            context_size: None,
            cost_usd: None,
            transcript_readonly: false,
            retired: false,
            parent_thread_id: None,
        }
    }

    #[cfg(test)]
    pub fn with_status(self, status: ThreadStatus) -> Self {
        Self {
            status: status.into(),
            ..self
        }
    }

    pub fn has_status(&self, status: ThreadStatus) -> bool {
        self.status == status.as_str()
    }
}

/// A titled one-off conversation against a repo checkout. Its thread's owner id is the scratch id.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScratchRow {
    pub id: String,
    pub repo_id: String,
    pub title: String,
    pub thread_id: String,
    pub engine: String,
    pub status: String,
    /// SQLite `datetime('now')`, UTC, `YYYY-MM-DD HH:MM:SS`.
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ThreadEventRow {
    pub id: i64,
    pub thread_id: String,
    pub seq: i64,
    pub kind: String,
    pub payload: String,
    /// SQLite `datetime('now')`, UTC, `YYYY-MM-DD HH:MM:SS`.
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRow {
    pub id: String,
    pub thread_id: String,
    pub status: String,
    pub tool: String,
    pub payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FindingRow {
    pub id: String,
    pub workspace_id: String,
    pub severity: String,
    pub title: String,
    pub file: Option<String>,
    pub line: Option<i64>,
    pub explanation: String,
    pub status: String,
    pub commit_sha: Option<String>,
    pub sent_to_thread_id: Option<String>,
}

/// Something a thread's agent linked to its work with `attach_entity`: a PR, a ticket, a doc.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EntityRow {
    pub id: i64,
    pub thread_id: String,
    pub name: String,
    pub url: String,
    /// SQLite `datetime('now')`, UTC, `YYYY-MM-DD HH:MM:SS`.
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PrRow {
    pub id: String,
    pub repo_id: Option<String>,
    pub number: i64,
    pub title: String,
    pub payload: String,
}

/// The last message from the user or end of a turn. Streamed output doesn't count, so a working
/// thread holds its place in the sidebar.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ThreadActivityRow {
    pub thread_id: String,
    pub created_at: String,
}

/// A window of a thread's transcript, oldest first.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ThreadEventPage {
    pub events: Vec<ThreadEventRow>,
    /// Whether rows older than this window exist.
    pub has_older: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PersistedSnapshot {
    pub settings: Vec<SettingRow>,
    pub repos: Vec<RepoRecord>,
    pub workspaces: Vec<WorkspaceRow>,
    pub threads: Vec<ThreadRow>,
    /// Newest first.
    pub scratches: Vec<ScratchRow>,
    /// When each thread last moved; threads that never have are left out.
    pub thread_activity: Vec<ThreadActivityRow>,
    pub approvals: Vec<ApprovalRow>,
    pub findings: Vec<FindingRow>,
    /// The open threads' attached entities, oldest first.
    pub entities: Vec<EntityRow>,
    pub pull_requests: Vec<PrRow>,
}
