//! The user's answer to an approval or question, and what follows from it.

use std::collections::BTreeMap;

use super::{ApprovalDecision, QUESTION_TOOL, apply_harness_effects, mark_answered, mark_resolved};
use crate::error::Result;
use crate::feedback::{Output, emit_approval_counts, toast_for_approval};
use crate::ipc::types::{ResolveApprovalResult, StateChangeKind};
use crate::state::AppState;

pub async fn resolve_approval(
    out: &Output,
    state: &AppState,
    id: &str,
    approved: bool,
    deny_reason: Option<String>,
) -> Result<ResolveApprovalResult> {
    let (row, payload) = mark_resolved(&state.store, id, approved, deny_reason.clone())?;
    let deny_message = if approved {
        None
    } else {
        deny_reason.or(payload.deny_reason.clone())
    };
    state
        .approvals
        .resolve(id, approved, deny_message.clone())
        .await?;
    let hints = apply_harness_effects(state, &row, &payload, approved)?;
    notify_approval_resolved(out, state);
    if !approved && let Some(reason) = deny_message.filter(|value| !value.trim().is_empty()) {
        let _ = state
            .engines
            .prompt(&row.thread_id, format!("Approval denied: {reason}"));
    }
    if let Some(payload) = toast_for_approval(&row.tool, approved) {
        out.toast(payload);
    }
    Ok(ResolveApprovalResult {
        focus_composer: hints.focus_composer,
    })
}

fn notify_approval_resolved(out: &Output, state: &AppState) {
    emit_approval_counts(out, state);
    state.notify(out, StateChangeKind::WorkspaceStatus);
}

pub fn answer_question(
    out: &Output,
    state: &AppState,
    id: &str,
    answers: BTreeMap<String, String>,
) -> Result<()> {
    mark_answered(&state.store, id, answers.clone())?;
    state
        .approvals
        .complete(id, ApprovalDecision::Answered { answers })?;
    notify_approval_resolved(out, state);
    Ok(())
}

pub async fn resolve_all_approvals(
    out: &Output,
    state: &AppState,
    thread_id: &str,
    approved: bool,
    deny_reason: Option<String>,
) -> Result<ResolveApprovalResult> {
    let pending = state.store.pending_approvals_for_thread(thread_id)?;
    let mut focus_composer = false;
    // Approving a question would answer it with nothing, so those wait for the user.
    for row in pending
        .into_iter()
        .filter(|row| !approved || row.tool != QUESTION_TOOL)
    {
        let result = resolve_approval(out, state, &row.id, approved, deny_reason.clone()).await?;
        focus_composer |= result.focus_composer;
    }
    Ok(ResolveApprovalResult { focus_composer })
}
