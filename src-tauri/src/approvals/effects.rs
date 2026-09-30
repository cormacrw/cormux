use crate::composer::persist_control_step;
use crate::engines::{AgentEvent, MessageRole, ToolCallStatus, ToolKind};
use crate::error::Result;
use crate::state::AppState;
use crate::store::types::ApprovalRow;

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
        let focus_composer = payload.effect.as_deref() == Some("execute_plan");
        return Ok(HarnessEffectHints { focus_composer });
    }

    match payload.effect.as_deref() {
        Some("send_to_lead") => cross_thread_suggestions(state, row, payload)?,
        Some("execute_plan") => {
            persist_control_step(&state.store, &row.thread_id, "Plan approved")?;
        }
        Some("delete_file") => {
            persist_control_step(
                &state.store,
                &row.thread_id,
                &format!("Deleted {}", payload.what),
            )?;
        }
        Some("migration") => {
            persist_control_step(&state.store, &row.thread_id, "Applied migration")?;
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
    let snap = state.store.snapshot()?;
    let reviewer = snap
        .threads
        .iter()
        .find(|thread| thread.id == reviewer_row.thread_id)
        .ok_or_else(|| crate::error::Error::Store("reviewer thread missing".into()))?;
    let lead = snap
        .threads
        .iter()
        .find(|thread| {
            thread.workspace_id == reviewer.workspace_id
                && thread.title.eq_ignore_ascii_case("lead")
        })
        .ok_or_else(|| crate::error::Error::Store("lead thread missing".into()))?;

    persist_control_step(
        &state.store,
        &reviewer_row.thread_id,
        "Sent suggestions to Lead",
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
    let payload_json = serde_json::to_string(&event)
        .map_err(|error| crate::error::Error::Store(error.to_string()))?;
    state
        .store
        .append_event(&lead.id, "message", &payload_json)?;

    let tool_event = AgentEvent::ToolCall {
        id: format!("review-{}", reviewer_row.id),
        title: "Suggestions sent".into(),
        name: None,
        kind: ToolKind::Other,
        status: ToolCallStatus::Completed,
        locations: Vec::new(),
        detail: Some(payload.what.clone()),
    };
    let tool_json = serde_json::to_string(&tool_event)
        .map_err(|error| crate::error::Error::Store(error.to_string()))?;
    state
        .store
        .append_event(&reviewer_row.thread_id, "tool", &tool_json)?;

    Ok(())
}
