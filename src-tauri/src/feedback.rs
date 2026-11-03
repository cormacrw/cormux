use std::sync::Arc;

use crate::ipc::types::{StateChangeKind, ToastPart, ToastRaisedPayload, ToastTone};
use crate::state::AppState;

/// How a workflow tells the user something happened. It is the only way out of the core for
/// these: `ipc::events::TauriOutput` sends them to the webview and the dock, and tests use
/// `RecordingOutput`.
pub trait UserOutput: Send + Sync {
    /// Prefer `AppState::notify`, which numbers changes and merges bursts of them.
    fn state_changed(&self, version: u64, kind: StateChangeKind);
    /// A toast carries its own payload, so it needs no `state_changed`: the webview showed it
    /// from `ToastRaised` and ignored the matching `Toast` state event anyway.
    fn toast(&self, payload: ToastRaisedPayload);
    fn dock_badge(&self, count: Option<i64>);

    fn toast_parts(&self, tone: ToastTone, parts: Vec<ToastPart>, workspace_id: Option<String>) {
        self.toast(ToastRaisedPayload {
            tone,
            parts,
            workspace_id,
            thread_id: None,
        });
    }
}

/// Shared so a workflow can hand it to the background tasks it spawns.
pub type Output = Arc<dyn UserOutput>;

pub fn emit_approval_counts(out: &Output, state: &AppState) {
    state.notify(out, StateChangeKind::ApprovalCounts);
    sync_dock_badge(out, state);
}

pub fn sync_dock_badge(out: &Output, state: &AppState) {
    let pending = pending_approvals(state);
    let count = if pending == 0 {
        None
    } else {
        Some(pending as i64)
    };
    out.dock_badge(count);
}

fn pending_approvals(state: &AppState) -> usize {
    let stored = state.store.pending_approval_count().unwrap_or(0);
    stored.max(state.approvals.pending_count())
}

/// The toast an approval earns once it's granted, if any. Only cross-thread effects get one;
/// an edit or delete already shows as resolved on its card.
pub fn toast_for_approval(tool: &str, approved: bool) -> Option<ToastRaisedPayload> {
    if !approved {
        return None;
    }
    let parts = match tool {
        "send_suggestions" | "review_suggestions" => vec![ToastPart::Text {
            value: "Review suggestions sent to Lead".into(),
        }],
        _ => return None,
    };
    Some(ToastRaisedPayload {
        tone: ToastTone::Ok,
        parts,
        workspace_id: None,
        thread_id: None,
    })
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub enum Emitted {
    State(StateChangeKind),
    Toast(ToastRaisedPayload),
    Badge(Option<i64>),
}

/// Keeps what a workflow told the user, in order, for tests to assert on.
#[cfg(test)]
#[derive(Default)]
pub struct RecordingOutput {
    emitted: std::sync::Mutex<Vec<Emitted>>,
    versions: std::sync::Mutex<Vec<u64>>,
}

#[cfg(test)]
impl RecordingOutput {
    /// The recorder, and the same thing as the `Output` a workflow takes.
    pub fn new() -> (Arc<Self>, Output) {
        let recording = Arc::new(Self::default());
        (recording.clone(), recording)
    }

    pub fn emitted(&self) -> Vec<Emitted> {
        self.emitted.lock().unwrap().clone()
    }

    pub fn state_changes(&self) -> Vec<StateChangeKind> {
        self.emitted()
            .into_iter()
            .filter_map(|event| match event {
                Emitted::State(kind) => Some(kind),
                _ => None,
            })
            .collect()
    }

    pub fn versions(&self) -> Vec<u64> {
        self.versions.lock().unwrap().clone()
    }

    /// Each toast's text, its parts joined.
    pub fn toasts(&self) -> Vec<String> {
        self.emitted()
            .into_iter()
            .filter_map(|event| match event {
                Emitted::Toast(payload) => Some(
                    payload
                        .parts
                        .iter()
                        .map(|part| match part {
                            ToastPart::Text { value } | ToastPart::Code { value } => value.as_str(),
                        })
                        .collect::<String>(),
                ),
                _ => None,
            })
            .collect()
    }
}

#[cfg(test)]
impl UserOutput for RecordingOutput {
    fn state_changed(&self, version: u64, kind: StateChangeKind) {
        self.versions.lock().unwrap().push(version);
        self.emitted.lock().unwrap().push(Emitted::State(kind));
    }

    fn toast(&self, payload: ToastRaisedPayload) {
        self.emitted.lock().unwrap().push(Emitted::Toast(payload));
    }

    fn dock_badge(&self, count: Option<i64>) {
        self.emitted.lock().unwrap().push(Emitted::Badge(count));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_counts_go_to_the_webview_and_then_the_dock() {
        let state = AppState::new(std::env::temp_dir());
        state.store.open_in_memory().unwrap();
        let (recording, out) = RecordingOutput::new();

        emit_approval_counts(&out, &state);

        assert!(matches!(
            recording.emitted().as_slice(),
            [
                Emitted::State(StateChangeKind::ApprovalCounts),
                Emitted::Badge(None)
            ]
        ));
    }

    #[test]
    fn approval_toast_copy() {
        let payload = toast_for_approval("send_suggestions", true).expect("toast");
        assert!(matches!(payload.tone, ToastTone::Ok));
        assert_eq!(payload.parts.len(), 1);
    }

    #[test]
    fn edits_deletes_and_denials_have_no_toast() {
        assert!(toast_for_approval("edit", false).is_none());
        assert!(toast_for_approval("edit", true).is_none());
        assert!(toast_for_approval("delete_file", true).is_none());
    }
}
