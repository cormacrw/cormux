use std::collections::BTreeMap;

use super::payload::{
    AgentQuestion, ApprovalPayload, QUESTION_TOOL, payload_from_permission, payload_with_questions,
};
use crate::engines::AgentEvent;
use crate::model::ApprovalStatus;
use crate::store::Store;
use crate::store::types::ApprovalRow;
use crate::transcript::Transcript;

pub fn record_pending_permission(
    store: &Store,
    transcript: &Transcript,
    thread_id: &str,
    event: &AgentEvent,
) -> Result<(), crate::error::Error> {
    record_pending(store, transcript, thread_id, event, |payload| payload)
}

pub fn record_pending_question(
    store: &Store,
    transcript: &Transcript,
    thread_id: &str,
    event: &AgentEvent,
    questions: Vec<AgentQuestion>,
) -> Result<(), crate::error::Error> {
    record_pending(store, transcript, thread_id, event, |payload| {
        payload_with_questions(payload, questions)
    })
}

fn record_pending(
    store: &Store,
    transcript: &Transcript,
    thread_id: &str,
    event: &AgentEvent,
    shape: impl FnOnce(ApprovalPayload) -> ApprovalPayload,
) -> Result<(), crate::error::Error> {
    let AgentEvent::Permission {
        id, auto_approved, ..
    } = event
    else {
        return Ok(());
    };
    if *auto_approved {
        return Ok(());
    }

    let (payload, tool_key) = payload_from_permission(event);
    let payload = shape(payload);
    let tool_key = if payload.questions.is_empty() {
        tool_key
    } else {
        QUESTION_TOOL.into()
    };
    let payload_json = serde_json::to_string(&payload)
        .map_err(|error| crate::error::Error::Store(error.to_string()))?;

    store.upsert_approval(&ApprovalRow {
        id: id.clone(),
        thread_id: thread_id.to_string(),
        status: ApprovalStatus::Pending.into(),
        tool: tool_key,
        payload: payload_json,
    })?;

    transcript.append(thread_id, event.clone())?;
    Ok(())
}

pub fn mark_resolved(
    store: &Store,
    id: &str,
    approved: bool,
    deny_reason: Option<String>,
) -> Result<(ApprovalRow, ApprovalPayload), crate::error::Error> {
    let status = if approved {
        ApprovalStatus::Approved
    } else {
        ApprovalStatus::Denied
    };
    finish(store, id, status, |payload| {
        if !approved {
            payload.deny_reason = deny_reason.filter(|value| !value.trim().is_empty());
        }
    })
}

pub fn mark_answered(
    store: &Store,
    id: &str,
    answers: BTreeMap<String, String>,
) -> Result<(ApprovalRow, ApprovalPayload), crate::error::Error> {
    finish(store, id, ApprovalStatus::Approved, |payload| {
        payload.answers = answers
    })
}

fn finish(
    store: &Store,
    id: &str,
    status: ApprovalStatus,
    apply: impl FnOnce(&mut ApprovalPayload),
) -> Result<(ApprovalRow, ApprovalPayload), crate::error::Error> {
    let row = store
        .get_approval(id)?
        .ok_or_else(|| crate::error::Error::Approval(format!("unknown approval {id}")))?;
    if row.status != ApprovalStatus::Pending.as_str() {
        return Err(crate::error::Error::Approval(
            "approval already decided".into(),
        ));
    }
    let mut payload: ApprovalPayload = serde_json::from_str(&row.payload).unwrap_or_default();
    payload.resolved_at_ms = Some(crate::time::unix_ms());
    apply(&mut payload);
    let payload_json = serde_json::to_string(&payload)
        .map_err(|error| crate::error::Error::Store(error.to_string()))?;
    let updated = ApprovalRow {
        status: status.into(),
        payload: payload_json,
        ..row
    };
    store.upsert_approval(&updated)?;
    Ok((updated, payload))
}
