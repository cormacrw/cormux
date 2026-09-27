use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use crate::workspace::WorkspaceLifecycle;

use super::types::StateChangeKind;

/// Low-volume core → UI notification. `version` is monotonic; a gap means the
/// webview should call `get_snapshot` and replace local state.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct StateChanged {
    pub version: u64,
    pub kind: StateChangeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceStatusChanged {
    pub version: u64,
    pub workspace_id: String,
    pub status: WorkspaceLifecycle,
}
