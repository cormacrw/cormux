use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::ipc::events::ToastRaised;
use crate::ipc::types::{StateChangeKind, ToastPart, ToastRaisedPayload, ToastTone};
use crate::state::AppState;

pub fn emit_approval_counts(app: &AppHandle, state: &AppState) {
    state.notify(app, StateChangeKind::ApprovalCounts);
    sync_dock_badge(app, state);
}

pub fn emit_toast_parts(
    app: &AppHandle,
    tone: ToastTone,
    parts: Vec<ToastPart>,
    workspace_id: Option<String>,
) {
    emit_toast(
        app,
        ToastRaisedPayload {
            tone,
            parts,
            workspace_id,
            thread_id: None,
        },
    );
}

/// A toast carries its own payload, so it needs no `StateChanged`: the webview showed it from
/// `ToastRaised` and ignored the matching `Toast` state event anyway.
pub fn emit_toast(app: &AppHandle, payload: ToastRaisedPayload) {
    let _ = ToastRaised { payload }.emit(app);
}

pub fn sync_dock_badge(app: &AppHandle, state: &AppState) {
    let pending = pending_approvals(state);
    let count = if pending == 0 {
        None
    } else {
        Some(pending as i64)
    };
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_badge_count(count);
    }
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
mod tests {
    use super::*;

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
