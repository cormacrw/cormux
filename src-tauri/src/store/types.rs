use serde::{Deserialize, Serialize};
use specta::Type;

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
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRow {
    pub id: String,
    pub repo_id: String,
    pub name: String,
    pub branch: String,
    pub worktree_path: String,
    pub status: String,
    pub created_at: String,
    pub summary: Option<String>,
    pub summary_at: Option<String>,
    pub summary_source: String,
    pub kind: Option<String>,
    pub pr_number: Option<i64>,
    pub pr_html_url: Option<String>,
    pub modified_files: i64,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ThreadRow {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub engine: String,
    pub session_id: Option<String>,
    pub status: String,
    pub used_tokens: Option<i64>,
    pub context_size: Option<i64>,
    pub cost_usd: Option<f64>,
    pub transcript_readonly: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PrRow {
    pub id: String,
    pub repo_id: Option<String>,
    pub number: i64,
    pub title: String,
    pub payload: String,
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
    pub timeline: Vec<ThreadEventRow>,
    pub approvals: Vec<ApprovalRow>,
    pub findings: Vec<FindingRow>,
    pub pull_requests: Vec<PrRow>,
}
