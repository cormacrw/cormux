use agent_client_protocol::schema::v1::{
    ContentBlock, PermissionOptionKind, RequestPermissionRequest, SessionUpdate, StopReason,
    ToolCall, ToolCallStatus as AcpStatus, ToolKind as AcpKind,
};

use crate::engines::events::{AgentEvent, MessageRole, PlanStep, ToolCallStatus, ToolKind};

pub fn map_session_update(update: &SessionUpdate) -> Option<AgentEvent> {
    match update {
        SessionUpdate::UserMessageChunk(chunk) => text_chunk(&chunk.content, MessageRole::User),
        SessionUpdate::AgentMessageChunk(chunk) => text_chunk(&chunk.content, MessageRole::Agent),
        SessionUpdate::AgentThoughtChunk(chunk) => text_chunk(&chunk.content, MessageRole::Thought),
        SessionUpdate::ToolCall(call) => Some(map_tool_call(call)),
        SessionUpdate::ToolCallUpdate(update) => {
            let id = update.tool_call_id.to_string();
            let title = update.fields.title.clone().unwrap_or_else(|| "tool".into());
            Some(AgentEvent::CurrentTool {
                id: Some(id.clone()),
                title: title.clone(),
            })
        }
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
    AgentEvent::Permission {
        id: format!("{}:{}", request.session_id, tool.tool_call_id),
        tool_call_id: Some(tool.tool_call_id.to_string()),
        title,
        tool_name: tool.fields.name.clone().unwrap_or_default(),
        kind,
        detail: None,
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
