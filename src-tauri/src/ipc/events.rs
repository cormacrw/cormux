use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::feedback::{Change, NotificationPayload, ToastRaisedPayload, UserOutput};

/// Low-volume core → UI notification. `version` is monotonic; a gap means the
/// webview should call `get_snapshot` and replace local state.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct StateChanged {
    pub version: u64,
    pub change: Change,
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

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct NotificationRaised {
    pub payload: NotificationPayload,
}

/// `UserOutput` for the real app: events to the webview, the count to the dock.
pub struct TauriOutput(pub AppHandle);

impl UserOutput for TauriOutput {
    fn state_changed(&self, version: u64, change: Change) {
        let _ = StateChanged { version, change }.emit(&self.0);
    }

    fn toast(&self, payload: ToastRaisedPayload) {
        let _ = ToastRaised { payload }.emit(&self.0);
    }

    fn notification(&self, payload: NotificationPayload) {
        let _ = NotificationRaised { payload }.emit(&self.0);
    }

    fn dock_badge(&self, count: Option<i64>) {
        if let Some(window) = self.0.get_webview_window("main") {
            let _ = window.set_badge_count(count);
        }
    }
}
