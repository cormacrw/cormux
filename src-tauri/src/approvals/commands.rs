use std::collections::BTreeMap;

use tauri::State;

use crate::error::Result;
use crate::feedback::Change;
use crate::state::AppState;

use super::Answer;

#[tauri::command]
#[specta::specta]
pub async fn resolve_approval(
    id: String,
    approved: bool,
    deny_reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<()> {
    let answer = if approved {
        Answer::Approve
    } else {
        Answer::Deny {
            reason: deny_reason,
        }
    };
    state.approvals.resolve(&id, answer)?;
    state.out.changed(Change::THREADS);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn answer_question(
    id: String,
    answers: BTreeMap<String, String>,
    state: State<'_, AppState>,
) -> Result<()> {
    state.approvals.resolve(&id, Answer::Answers(answers))?;
    state.out.changed(Change::THREADS);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn resolve_all_approvals(
    thread_id: String,
    approved: bool,
    deny_reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<()> {
    let resolved = state
        .approvals
        .resolve_all(&thread_id, approved, deny_reason);
    state.out.changed(Change::THREADS);
    resolved
}
