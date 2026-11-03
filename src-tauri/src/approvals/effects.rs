use crate::composer::persist_agent_event;
use crate::engines::{AgentEvent, MessageRole};
use crate::error::Result;
use crate::state::AppState;
use crate::store::types::ApprovalRow;
use crate::timeline::{TimelineStep, record_step};

use super::payload::ApprovalPayload;

pub struct HarnessEffectHints {
    pub focus_composer: bool,
}

pub fn apply_harness_effects(
    state: &AppState,
    row: &ApprovalRow,
    payload: &ApprovalPayload,
    approved: bool,
) -> Result<HarnessEffectHints> {
    if !approved {
        return Ok(HarnessEffectHints {
            focus_composer: false,
        });
    }

    match payload.effect.as_deref() {
        Some("send_to_lead") => cross_thread_suggestions(state, row, payload)?,
        Some("delete_file") => {
            record_step(
                &state.store,
                &row.thread_id,
                TimelineStep::FileDeleted {
                    path: payload.what.clone(),
                },
            )?;
        }
        Some("migration") => {
            record_step(&state.store, &row.thread_id, TimelineStep::MigrationApplied)?;
        }
        _ => {}
    }

    Ok(HarnessEffectHints {
        focus_composer: false,
    })
}

fn cross_thread_suggestions(
    state: &AppState,
    reviewer_row: &ApprovalRow,
    payload: &ApprovalPayload,
) -> Result<()> {
    let reviewer = state
        .store
        .thread_by_id(&reviewer_row.thread_id)?
        .ok_or_else(|| crate::error::Error::Store("reviewer thread missing".into()))?;
    let lead = state
        .store
        .threads_for_workspace(&reviewer.workspace_id)?
        .into_iter()
        .find(|thread| thread.title.eq_ignore_ascii_case("lead"))
        .ok_or_else(|| crate::error::Error::Store("lead thread missing".into()))?;

    record_step(
        &state.store,
        &reviewer_row.thread_id,
        TimelineStep::SuggestionsForwarded,
    )?;

    let thought = if payload.why.is_empty() {
        format!("Reviewer suggests changes to {}.", payload.what)
    } else {
        format!("Reviewer suggests: {}", payload.why)
    };
    let event = AgentEvent::MessageChunk {
        role: MessageRole::Thought,
        text: thought,
    };
    persist_agent_event(&state.store, &lead.id, &event)?;

    record_step(
        &state.store,
        &reviewer_row.thread_id,
        TimelineStep::SuggestionsSent {
            approval_id: reviewer_row.id.clone(),
            what: payload.what.clone(),
        },
    )
}
