use super::protocol::{CanUseTool, Event};
use crate::engines::events::{
    AgentEvent, MessageRole, PlanStep, ToolCallStatus, ToolKind,
};

pub fn map_claude_event(event: &Event) -> Option<AgentEvent> {
    match event {
        Event::Init { session_id } => Some(AgentEvent::SessionStarted {
            session_id: session_id.clone(),
        }),
        Event::Assistant { text } if !text.is_empty() => Some(AgentEvent::MessageChunk {
            role: MessageRole::Agent,
            text: text.clone(),
        }),
        Event::Assistant { .. } => None,
        Event::CanUseTool(req) => Some(map_permission(req)),
        Event::Result {
            is_error,
            subtype,
            result,
            ..
        } => Some(AgentEvent::TurnEnd {
            stop_reason: if *is_error {
                subtype.clone().unwrap_or_else(|| "error".into())
            } else {
                "end_turn".into()
            },
            error: if *is_error { result.clone() } else { None },
        }),
        Event::ControlResponse { .. } | Event::Other { .. } => None,
    }
}

fn map_permission(req: &CanUseTool) -> AgentEvent {
    let kind = ToolKind::from_claude_tool(&req.tool_name);
    let detail = req.input.get("command").and_then(|v| v.as_str()).map(str::to_string)
        .or_else(|| req.input.get("file_path").and_then(|v| v.as_str()).map(str::to_string));
    AgentEvent::Permission {
        id: req.request_id.clone(),
        tool_call_id: req.tool_use_id.clone(),
        title: format!("Use {}", req.tool_name),
        tool_name: req.tool_name.clone(),
        kind,
        detail,
        auto_approved: false,
    }
}

pub fn tool_event_from_permission(req: &CanUseTool, status: ToolCallStatus) -> AgentEvent {
    let kind = ToolKind::from_claude_tool(&req.tool_name);
    AgentEvent::ToolCall {
        id: req
            .tool_use_id
            .clone()
            .unwrap_or_else(|| req.request_id.clone()),
        title: req.tool_name.clone(),
        name: Some(req.tool_name.clone()),
        kind,
        status,
        locations: Vec::new(),
        detail: req.decision_reason.clone(),
    }
}

#[allow(dead_code)]
pub fn plan_from_titles(titles: &[String]) -> AgentEvent {
    AgentEvent::Plan {
        entries: titles
            .iter()
            .map(|content| PlanStep {
                content: content.clone(),
                status: "pending".into(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_assistant_and_permission() {
        let mapped = map_claude_event(&Event::Assistant {
            text: "hello".into(),
        })
        .unwrap();
        assert!(matches!(
            mapped,
            AgentEvent::MessageChunk {
                role: MessageRole::Agent,
                ..
            }
        ));

        let mapped = map_claude_event(&Event::CanUseTool(CanUseTool {
            request_id: "req".into(),
            tool_name: "Bash".into(),
            input: json!({"command": "echo hi"}),
            tool_use_id: Some("t1".into()),
            decision_reason: None,
        }))
        .unwrap();
        match mapped {
            AgentEvent::Permission {
                kind,
                detail,
                auto_approved,
                ..
            } => {
                assert_eq!(kind, ToolKind::Execute);
                assert_eq!(detail.as_deref(), Some("echo hi"));
                assert!(!auto_approved);
            }
            other => panic!("{other:?}"),
        }
    }
}
