use super::payload::{
    ApprovalPayload, payload_from_permission, payload_with_option_labels, payload_with_plan,
};
use crate::engines::AgentEvent;
use crate::store::Store;
use crate::store::types::ApprovalRow;

pub fn record_pending_permission(
    store: &Store,
    thread_id: &str,
    event: &AgentEvent,
    option_labels: Option<(Option<String>, Option<String>)>,
) -> Result<(), crate::error::Error> {
    record_pending(store, thread_id, event, |payload| match option_labels {
        Some((ok, no)) => payload_with_option_labels(payload, ok, no),
        None => payload,
    })
}

/// An `ExitPlanMode` call, recorded with its plan for the Plan tab.
pub fn record_pending_plan(
    store: &Store,
    thread_id: &str,
    event: &AgentEvent,
    plan: String,
) -> Result<(), crate::error::Error> {
    record_pending(store, thread_id, event, |payload| {
        payload_with_plan(payload, plan)
    })
}

fn record_pending(
    store: &Store,
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
    let payload_json = serde_json::to_string(&payload)
        .map_err(|error| crate::error::Error::Store(error.to_string()))?;

    store.upsert_approval(&ApprovalRow {
        id: id.clone(),
        thread_id: thread_id.to_string(),
        status: "pending".into(),
        tool: tool_key,
        payload: payload_json,
    })?;

    persist_permission_event(store, thread_id, event)?;
    Ok(())
}

pub fn mark_resolved(
    store: &Store,
    id: &str,
    approved: bool,
    deny_reason: Option<String>,
) -> Result<(ApprovalRow, ApprovalPayload), crate::error::Error> {
    let row = store
        .get_approval(id)?
        .ok_or_else(|| crate::error::Error::Approval(format!("unknown approval {id}")))?;
    if row.status != "pending" {
        return Err(crate::error::Error::Approval(
            "approval already decided".into(),
        ));
    }
    let mut payload: ApprovalPayload = serde_json::from_str(&row.payload).unwrap_or_default();
    payload.resolved_at_ms = Some(chrono_ms());
    if !approved {
        payload.deny_reason = deny_reason.filter(|value| !value.trim().is_empty());
    }
    let payload_json = serde_json::to_string(&payload)
        .map_err(|error| crate::error::Error::Store(error.to_string()))?;
    let status = if approved { "approved" } else { "denied" };
    let updated = ApprovalRow {
        status: status.into(),
        payload: payload_json,
        ..row
    };
    store.upsert_approval(&updated)?;
    Ok((updated, payload))
}

/// Closes a request nobody answered, such as a plan the user left plan mode on.
pub fn mark_dismissed(store: &Store, id: &str) -> Result<ApprovalRow, crate::error::Error> {
    let row = store
        .get_approval(id)?
        .ok_or_else(|| crate::error::Error::Approval(format!("unknown approval {id}")))?;
    let mut payload: ApprovalPayload = serde_json::from_str(&row.payload).unwrap_or_default();
    payload.resolved_at_ms = Some(chrono_ms());
    let payload_json = serde_json::to_string(&payload)
        .map_err(|error| crate::error::Error::Store(error.to_string()))?;
    let updated = ApprovalRow {
        status: "dismissed".into(),
        payload: payload_json,
        ..row
    };
    store.upsert_approval(&updated)?;
    Ok(updated)
}

fn persist_permission_event(
    store: &Store,
    thread_id: &str,
    event: &AgentEvent,
) -> Result<(), crate::error::Error> {
    let payload = serde_json::to_string(event)
        .map_err(|error| crate::error::Error::Store(error.to_string()))?;
    store.append_event(thread_id, "permission", &payload)?;
    Ok(())
}

fn chrono_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}
