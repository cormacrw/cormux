use agent_client_protocol::schema::v1::{
    ContentBlock, PermissionOptionKind, RequestPermissionRequest, SessionConfigId,
    SessionConfigKind, SessionConfigOption, SessionConfigOptionCategory,
    SessionConfigSelectOptions, SessionConfigValueId, SessionUpdate, StopReason, ToolCall,
    ToolCallStatus as AcpStatus, ToolKind as AcpKind,
};

use crate::engines::events::{AgentEvent, MessageRole, PlanStep, ToolCallStatus, ToolKind};
use crate::engines::models::ModelOption;

/// The session's model picker, from the config options the agent sent with the session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelConfig {
    pub config_id: SessionConfigId,
    /// The value the session started on, which "Default" goes back to.
    pub initial: SessionConfigValueId,
    pub options: Vec<ModelOption>,
}

pub fn model_config(options: &[SessionConfigOption]) -> Option<ModelConfig> {
    let option = options
        .iter()
        .find(|option| matches!(option.category, Some(SessionConfigOptionCategory::Model)))?;
    let SessionConfigKind::Select(select) = &option.kind else {
        return None;
    };
    let values: Vec<_> = match &select.options {
        SessionConfigSelectOptions::Ungrouped(values) => values.iter().collect(),
        SessionConfigSelectOptions::Grouped(groups) => {
            groups.iter().flat_map(|group| &group.options).collect()
        }
        _ => Vec::new(),
    };
    Some(ModelConfig {
        config_id: option.id.clone(),
        initial: select.current_value.clone(),
        options: values
            .into_iter()
            .map(|value| ModelOption {
                id: value.value.to_string(),
                label: value.name.clone(),
            })
            .collect(),
    })
}

pub fn map_session_update(update: &SessionUpdate) -> Option<AgentEvent> {
    match update {
        SessionUpdate::UserMessageChunk(chunk) => text_chunk(&chunk.content, MessageRole::User),
        SessionUpdate::AgentMessageChunk(chunk) => text_chunk(&chunk.content, MessageRole::Agent),
        SessionUpdate::AgentThoughtChunk(chunk) => text_chunk(&chunk.content, MessageRole::Thought),
        SessionUpdate::ToolCall(call) => Some(map_tool_call(call)),
        SessionUpdate::ToolCallUpdate(update) => Some(AgentEvent::ToolCallUpdate {
            id: update.tool_call_id.to_string(),
            title: update.fields.title.clone(),
            kind: update.fields.kind.map(map_tool_kind),
            status: update.fields.status.map(map_status),
            locations: update
                .fields
                .locations
                .iter()
                .flatten()
                .map(|loc| loc.path.display().to_string())
                .collect(),
        }),
        SessionUpdate::Plan(plan) => Some(AgentEvent::Plan {
            entries: plan
                .entries
                .iter()
                .map(|entry| PlanStep {
                    content: entry.content.clone(),
                    status: match entry.status {
                        agent_client_protocol::schema::v1::PlanEntryStatus::Pending => {
                            "pending".into()
                        }
                        agent_client_protocol::schema::v1::PlanEntryStatus::InProgress => {
                            "in_progress".into()
                        }
                        agent_client_protocol::schema::v1::PlanEntryStatus::Completed => {
                            "completed".into()
                        }
                        _ => "pending".into(),
                    },
                })
                .collect(),
        }),
        SessionUpdate::UsageUpdate(usage) => Some(AgentEvent::Usage {
            used_tokens: usage.used,
            context_size: usage.size,
            cost_usd: usage.cost.as_ref().map(|cost| cost.amount),
        }),
        _ => None,
    }
}

pub fn map_permission_request(request: &RequestPermissionRequest) -> AgentEvent {
    let tool = &request.tool_call;
    let title = tool
        .fields
        .title
        .clone()
        .unwrap_or_else(|| "Needs approval".into());
    let kind = tool
        .fields
        .kind
        .map(map_tool_kind)
        .unwrap_or(ToolKind::Other);
    let detail = tool
        .fields
        .locations
        .as_ref()
        .and_then(|locs| locs.first())
        .map(|loc| loc.path.display().to_string())
        .or_else(|| tool.fields.name.clone());
    AgentEvent::Permission {
        id: format!("{}:{}", request.session_id, tool.tool_call_id),
        tool_call_id: Some(tool.tool_call_id.to_string()),
        title,
        tool_name: tool.fields.name.clone().unwrap_or_default(),
        kind,
        detail,
        auto_approved: false,
    }
}

pub fn map_stop_reason(reason: &StopReason) -> AgentEvent {
    let stop_reason = match reason {
        StopReason::EndTurn => "end_turn",
        StopReason::Cancelled => "cancelled",
        StopReason::MaxTokens => "max_tokens",
        StopReason::MaxTurnRequests => "max_turn_requests",
        StopReason::Refusal => "refusal",
        _ => "other",
    };
    AgentEvent::TurnEnd {
        stop_reason: stop_reason.into(),
        error: None,
    }
}

pub fn permission_option_labels(
    request: &RequestPermissionRequest,
) -> (Option<String>, Option<String>) {
    let allow = request
        .options
        .iter()
        .find(|opt| opt.kind == PermissionOptionKind::AllowOnce)
        .map(|opt| opt.name.clone());
    let deny = request
        .options
        .iter()
        .find(|opt| opt.kind == PermissionOptionKind::RejectOnce)
        .map(|opt| opt.name.clone());
    (allow, deny)
}

pub fn pick_permission_option(
    request: &RequestPermissionRequest,
    approved: bool,
) -> Option<String> {
    let want = if approved {
        PermissionOptionKind::AllowOnce
    } else {
        PermissionOptionKind::RejectOnce
    };
    request
        .options
        .iter()
        .find(|opt| opt.kind == want)
        .or_else(|| request.options.first())
        .map(|opt| opt.option_id.to_string())
}

fn map_tool_call(call: &ToolCall) -> AgentEvent {
    AgentEvent::ToolCall {
        id: call.tool_call_id.to_string(),
        title: call.title.clone(),
        name: call.name.clone(),
        kind: map_tool_kind(call.kind),
        status: map_status(call.status),
        locations: call
            .locations
            .iter()
            .map(|loc| loc.path.display().to_string())
            .collect(),
        detail: None,
    }
}

fn map_tool_kind(kind: AcpKind) -> ToolKind {
    match kind {
        AcpKind::Read => ToolKind::Read,
        AcpKind::Edit => ToolKind::Edit,
        AcpKind::Delete => ToolKind::Delete,
        AcpKind::Move => ToolKind::Move,
        AcpKind::Search => ToolKind::Search,
        AcpKind::Execute => ToolKind::Execute,
        AcpKind::Think => ToolKind::Think,
        AcpKind::Fetch => ToolKind::Fetch,
        _ => ToolKind::Other,
    }
}

fn map_status(status: AcpStatus) -> ToolCallStatus {
    match status {
        AcpStatus::Pending => ToolCallStatus::Pending,
        AcpStatus::InProgress => ToolCallStatus::InProgress,
        AcpStatus::Completed => ToolCallStatus::Completed,
        AcpStatus::Failed => ToolCallStatus::Failed,
        _ => ToolCallStatus::Pending,
    }
}

fn text_chunk(block: &ContentBlock, role: MessageRole) -> Option<AgentEvent> {
    match block {
        ContentBlock::Text(text) if !text.text.is_empty() => Some(AgentEvent::MessageChunk {
            role,
            text: text.text.clone(),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_config_reads_the_model_category() {
        let options: Vec<SessionConfigOption> = serde_json::from_value(serde_json::json!([
            {
                "id": "mode", "name": "Mode", "category": "mode", "type": "select",
                "currentValue": "agent", "options": [{ "value": "agent", "name": "Agent" }]
            },
            {
                "id": "model", "name": "Model", "category": "model", "type": "select",
                "currentValue": "gpt-5",
                "options": [
                    { "value": "gpt-5", "name": "GPT-5" },
                    { "value": "sonnet-4", "name": "Sonnet 4" }
                ]
            }
        ]))
        .unwrap();
        let config = model_config(&options).unwrap();
        assert_eq!(config.config_id.to_string(), "model");
        assert_eq!(config.initial.to_string(), "gpt-5");
        assert_eq!(
            config.options,
            [
                ModelOption {
                    id: "gpt-5".into(),
                    label: "GPT-5".into()
                },
                ModelOption {
                    id: "sonnet-4".into(),
                    label: "Sonnet 4".into()
                },
            ]
        );
        assert!(model_config(&options[..1]).is_none());
    }

    #[test]
    fn tool_call_update_carries_late_locations() {
        let update: SessionUpdate = serde_json::from_value(serde_json::json!({
            "sessionUpdate": "tool_call_update",
            "toolCallId": "edit-1",
            "status": "completed",
            "locations": [{ "path": "/repo/src/app.ts" }]
        }))
        .unwrap();
        assert_eq!(
            map_session_update(&update),
            Some(AgentEvent::ToolCallUpdate {
                id: "edit-1".into(),
                title: None,
                kind: None,
                status: Some(ToolCallStatus::Completed),
                locations: vec!["/repo/src/app.ts".into()],
            })
        );
    }
}
