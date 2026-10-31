use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use super::types::{StateChangeKind, ToastRaisedPayload};

/// Low-volume core → UI notification. `version` is monotonic; a gap means the
/// webview should call `get_snapshot` and replace local state.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct StateChanged {
    pub version: u64,
    pub kind: StateChangeKind,
}

/// A fresh memory reading. Carries its own data, so it needs no version or refetch.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct MemorySampled {
    pub sample: crate::metrics::MemorySample,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct ToastRaised {
    pub payload: ToastRaisedPayload,
}
