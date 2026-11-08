//! The user's answer to an approval or question, and what follows from it.

use std::collections::BTreeMap;

use super::{ApprovalDecision, QUESTION_TOOL, mark_answered, mark_resolved};
use crate::error::Result;
use crate::feedback::Change;
use crate::state::AppState;

pub async fn resolve_approval(
    state: &AppState,
    id: &str,
    approved: bool,
    deny_reason: Option<String>,
) -> Result<()> {
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
    state.out.changed(Change::THREADS);
    if !approved && let Some(reason) = deny_message.filter(|value| !value.trim().is_empty()) {
        let _ = state
            .engines
            .prompt(&row.thread_id, format!("Approval denied: {reason}"));
    }
    Ok(())
}

pub fn answer_question(
    state: &AppState,
    id: &str,
    answers: BTreeMap<String, String>,
) -> Result<()> {
    mark_answered(&state.store, id, answers.clone())?;
    state
        .approvals
        .complete(id, ApprovalDecision::Answered { answers })?;
    state.out.changed(Change::THREADS);
    Ok(())
}

pub async fn resolve_all_approvals(
    state: &AppState,
    thread_id: &str,
    approved: bool,
    deny_reason: Option<String>,
) -> Result<()> {
    let pending = state.store.pending_approvals_for_thread(thread_id)?;
    // Approving a question would answer it with nothing, so those wait for the user.
    for row in pending
        .into_iter()
        .filter(|row| !approved || row.tool != QUESTION_TOOL)
    {
        resolve_approval(state, &row.id, approved, deny_reason.clone()).await?;
    }
    Ok(())
}
