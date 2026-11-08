use std::collections::BTreeMap;

use tauri::State;

use crate::error::Result;
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
    answer_and_notify(&state, &id, answer)
}

#[tauri::command]
#[specta::specta]
pub async fn answer_question(
    id: String,
    answers: BTreeMap<String, String>,
    state: State<'_, AppState>,
) -> Result<()> {
    answer_and_notify(&state, &id, Answer::Answers(answers))
}

/// A refused answer still changes the card (it stops asking), so the webview hears either way.
fn answer_and_notify(state: &AppState, id: &str, answer: Answer) -> Result<()> {
    let resolved = state.approvals.resolve(id, answer);
    state.out.changed();
    resolved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refused_answer_still_reaches_the_webview() {
        let (state, recording) =
            AppState::recorded(crate::paths::DataDir::new(std::env::temp_dir()));
        state
            .store
            .seed_workspace("w1", crate::model::WorkspaceLifecycle::Idle);
        state
            .store
            .seed_thread("t1", "w1", crate::model::ThreadStatus::Running);
        state
            .store
            .insert_pending_approval(
                "a1",
                "t1",
                "execute",
                &crate::model::ApprovalPayload::titled("Use Bash"),
            )
            .unwrap();

        assert!(answer_and_notify(&state, "a1", Answer::Approve).is_err());
        assert!(!recording.state_changes().is_empty());
    }
}
