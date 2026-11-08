use serde::{Deserialize, Serialize};
use specta::Type;

use crate::git::WorktreeDiff;

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

pub use crate::teardown::{TeardownInput, TeardownPreview};
