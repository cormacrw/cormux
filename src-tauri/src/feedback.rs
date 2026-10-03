use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::ipc::events::{StateChanged, ToastRaised};
use crate::ipc::types::{StateChangeKind, ToastPart, ToastRaisedPayload, ToastTone};
use crate::state::AppState;

pub fn emit_approval_counts(app: &AppHandle, state: &AppState) {
    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::ApprovalCounts,
    }
    .emit(app);
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

pub fn emit_toast(app: &AppHandle, payload: ToastRaisedPayload) {
    let _ = ToastRaised { payload }.emit(app);
    let version = state_bump(app);
    let _ = StateChanged {
        version,
        kind: StateChangeKind::Toast,
    }
    .emit(app);
}

fn state_bump(app: &AppHandle) -> u64 {
    app.state::<AppState>().bump_event_version()
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
    let stored = state
        .store
        .snapshot()
        .map(|snap| {
            snap.approvals
                .iter()
                .filter(|row| row.status == "pending")
                .count()
        })
        .unwrap_or(0);
    stored.max(state.approvals.pending_count())
}

pub fn toast_for_approval(tool: &str, approved: bool) -> Option<ToastRaisedPayload> {
    if !approved {
        return None;
    }
    let (tone, parts) = match tool {
        "edit" | "delete_file" => (
            ToastTone::Ok,
            vec![ToastPart::Text {
                value: "Deleted legacy-cookie.ts in worktree".into(),
            }],
        ),
        "send_suggestions" | "review_suggestions" => (
            ToastTone::Ok,
            vec![ToastPart::Text {
                value: "Review suggestions sent to Lead".into(),
            }],
        ),
        _ => return None,
    };
    Some(ToastRaisedPayload {
        tone,
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
    fn denied_approval_has_no_toast() {
        assert!(toast_for_approval("edit", false).is_none());
    }
}
