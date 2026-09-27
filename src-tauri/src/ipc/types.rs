use serde::{Deserialize, Serialize};
use specta::Type;

use crate::git::{BehindUpdate, WorktreeDiff};
use crate::metrics::MemorySample;
use crate::store::types::PersistedSnapshot;
use crate::workspace::{WorkspaceLifecycle, WorkspaceRecord};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub version: u64,
    pub view: AppView,
    pub persisted: PersistedSnapshot,
    pub workspaces: Vec<WorkspaceRecord>,
    pub memory: Option<MemorySample>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AppView {
    Homebase,
    Workspace { id: String },
    Settings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentChunk {
    pub thread_id: String,
    pub text: String,
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
    pub path: String,
    pub diff: Option<WorktreeDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum StateChangeKind {
    WorkspaceStatus,
    ApprovalCounts,
    PrSync,
    Toast,
    BehindCounts,
    Metrics,
    Environment,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceStatusPayload {
    pub version: u64,
    pub workspace: WorkspaceRecord,
    pub status: WorkspaceLifecycle,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BehindCountsPayload {
    pub version: u64,
    pub updates: Vec<BehindUpdate>,
}
