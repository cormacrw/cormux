use serde::{Deserialize, Serialize};
use specta::Type;

use crate::app::WorkspaceAppRuntime;
use crate::git::WorktreeDiff;
use crate::metrics::MemorySample;
use crate::store::types::PersistedSnapshot;
use crate::workspace_git::WorkspaceGitRuntime;
use crate::workspaces::WorkspaceRecord;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub version: u64,
    pub persisted: PersistedSnapshot,
    /// Every live workspace, in the order they were created.
    pub workspaces: Vec<WorkspaceRecord>,
    pub workspace_git: Vec<WorkspaceGitRuntime>,
    pub memory: Option<MemorySample>,
    /// In-memory broker queue (may exceed persisted pending rows).
    pub pending_live_approvals: usize,
    /// True when a PAT is stored or `gh auth token` succeeds.
    pub github_auth_configured: bool,
    /// True when a ClickUp API key is saved. Every ClickUp surface hides without one.
    pub clickup_configured: bool,
    pub workspace_apps: Vec<WorkspaceAppRuntime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PtyChunk {
    pub workspace_id: String,
    pub line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DiffUpdate {
    pub workspace_id: String,
    pub diff: Option<WorktreeDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceResult {
    pub workspace_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SwitchWorkspaceBranchInput {
    pub workspace_id: String,
    pub branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceBranchInput {
    pub workspace_id: String,
    pub branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ControlWorkspaceAppInput {
    pub workspace_id: String,
    pub action: WorkspaceAppControlAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceAppControlAction {
    Run,
    Restart,
    Stop,
    Clear,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SetSettingInput {
    pub key: String,
    pub value: String,
}

pub use crate::engines::AgentEvent;
pub use crate::teardown::{TeardownInput, TeardownPreview};
