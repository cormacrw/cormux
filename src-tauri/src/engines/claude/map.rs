use serde_json::Value;

use super::protocol::{AssistantBlock, CanUseTool, Event};
use crate::engines::events::{AgentEvent, MessageRole, PlanStep, ToolCallStatus, ToolKind};

/// Timeline events for one CLI event. An assistant message can hold thinking, text and
/// several tool calls, so this returns them all in order.
pub fn map_claude_event(event: &Event) -> Vec<AgentEvent> {
    match event {
        Event::Init { session_id, .. } => vec![AgentEvent::SessionStarted {
            session_id: session_id.clone(),
        }],
        Event::Assistant { blocks, parent_id } => blocks
            .iter()
            .filter_map(|block| map_block(block, parent_id.as_deref()))
            .collect(),
        Event::ToolResults(results) => results
            .iter()
            .map(|result| AgentEvent::ToolCallUpdate {
                id: result.tool_use_id.clone(),
                title: None,
                kind: None,
                status: Some(if result.is_error {
                    ToolCallStatus::Failed
                } else {
                    ToolCallStatus::Completed
                }),
                locations: Vec::new(),
                output: result.output.clone(),
            })
            .collect(),
        Event::CanUseTool(req) => vec![map_permission(req)],
        Event::Result {
            is_error,
            subtype,
            result,
            ..
        } => vec![AgentEvent::TurnEnd {
            stop_reason: if *is_error {
                subtype.clone().unwrap_or_else(|| "error".into())
            } else {
                "end_turn".into()
            },
            error: if *is_error { result.clone() } else { None },
        }],
        Event::McpMessage { .. } | Event::ControlResponse { .. } | Event::Other { .. } => {
            Vec::new()
        }
    }
}

fn map_block(block: &AssistantBlock, parent_id: Option<&str>) -> Option<AgentEvent> {
    match block {
        AssistantBlock::Text(text) if !text.is_empty() => Some(AgentEvent::MessageChunk {
            role: MessageRole::Agent,
            text: text.clone(),
        }),
        AssistantBlock::Thinking(text) if !text.trim().is_empty() => {
            Some(AgentEvent::MessageChunk {
                role: MessageRole::Thought,
                text: text.clone(),
            })
        }
        AssistantBlock::ToolUse { input, name, .. } if name == "TodoWrite" => {
            Some(todo_plan(input))
        }
        AssistantBlock::ToolUse { id, name, input } => Some(tool_call(id, name, input, parent_id)),
        _ => None,
    }
}

/// Claude's todo list is its plan, re-sent whole each time a step moves on.
fn todo_plan(input: &Value) -> AgentEvent {
    let todos = input.get("todos").and_then(Value::as_array);
    AgentEvent::Plan {
        entries: todos
            .into_iter()
            .flatten()
            .map(|todo| {
                let field = |key: &str| todo.get(key).and_then(Value::as_str).unwrap_or_default();
                PlanStep {
                    content: field("content").to_string(),
                    status: field("status").to_string(),
                }
            })
            .collect(),
    }
}

/// A tool Claude started. Tools it runs without asking never reach `can_use_tool`, so this
/// is the only place they show up; a later tool result marks them done.
fn tool_call(id: &str, name: &str, input: &Value, parent_id: Option<&str>) -> AgentEvent {
    let kind = ToolKind::from_claude_tool(name);
    let field = |key: &str| input.get(key).and_then(Value::as_str).map(str::to_string);
    let path = field("file_path").or_else(|| field("notebook_path"));
    // The timeline shows a shell step by its command.
    let (title, detail) = match kind {
        ToolKind::Execute => (
            field("command").unwrap_or_else(|| name.to_string()),
            field("description"),
        ),
        _ => (
            name.to_string(),
            path.clone()
                .or_else(|| field("path"))
                .or_else(|| field("pattern"))
                .or_else(|| field("url"))
                .or_else(|| field("query"))
                .or_else(|| field("skill"))
                .or_else(|| field("description")),
        ),
    };
    AgentEvent::ToolCall {
        parent_id: parent_id.map(str::to_string),
        id: id.to_string(),
        title,
        name: Some(name.to_string()),
        kind,
        status: ToolCallStatus::InProgress,
        locations: path.into_iter().collect(),
        detail,
    }
}

pub fn map_permission(req: &CanUseTool) -> AgentEvent {
    let kind = ToolKind::from_claude_tool(&req.tool_name);
    let detail = req
        .input
        .get("command")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| {
            req.input
                .get("file_path")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        });
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
        parent_id: None,
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

pub fn usage_from_result(event: &Event) -> Option<AgentEvent> {
    let Event::Result {
        used_tokens,
        context_size,
        cost_usd,
        ..
    } = event
    else {
        return None;
    };
    let used = (*used_tokens)?;
    Some(AgentEvent::Usage {
        used_tokens: used,
        // 0 is "not reported": an unknown window shouldn't read as a full context.
        context_size: context_size.unwrap_or(0),
        cost_usd: cost_usd.as_ref().and_then(|value| value.parse().ok()),
    })
}

/// Claude reports cost as a running total for its process, which starts over when a session is
/// resumed in a new one. Given the thread's saved total, the process total last seen and the new
/// one, returns the thread's new total and the process total to remember.
pub fn add_turn_cost(thread_total: f64, process_seen: f64, process_total: f64) -> (f64, f64) {
    let added = (process_total - process_seen).max(0.0);
    (thread_total + added, process_total)
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
    fn a_todo_list_is_the_plan_with_its_progress() {
        let block = AssistantBlock::ToolUse {
            id: "t".into(),
            name: "TodoWrite".into(),
            input: json!({ "todos": [{ "content": "Fix it", "status": "in_progress" }] }),
        };
        assert_eq!(
            map_block(&block, None),
            Some(AgentEvent::Plan {
                entries: vec![PlanStep {
                    content: "Fix it".into(),
                    status: "in_progress".into(),
                }],
            })
        );
    }

    #[test]
    fn maps_assistant_blocks_and_tool_results_in_order() {
        use super::super::protocol::decode_event;
        let assistant = decode_event(
            &json!({
                "type": "assistant",
                "message": {"content": [
                    {"type": "thinking", "thinking": "Look at the readme first."},
                    {"type": "text", "text": "Checking."},
                    {"type": "tool_use", "id": "toolu_1", "name": "Bash",
                     "input": {"command": "ls -la", "description": "List files"}},
                    {"type": "tool_use", "id": "toolu_2", "name": "Read",
                     "input": {"file_path": "/repo/README.md"}}
                ]}
            })
            .to_string(),
        )
        .unwrap();
        let mapped = map_claude_event(&assistant);
        assert_eq!(mapped.len(), 4);
        assert!(matches!(
            &mapped[0],
            AgentEvent::MessageChunk { role: MessageRole::Thought, text } if text == "Look at the readme first."
        ));
        assert!(matches!(
            &mapped[1],
            AgentEvent::MessageChunk { role: MessageRole::Agent, text } if text == "Checking."
        ));
        match &mapped[2] {
            AgentEvent::ToolCall {
                id,
                title,
                kind,
                status,
                detail,
                ..
            } => {
                assert_eq!(id, "toolu_1");
                assert_eq!(title, "ls -la");
                assert_eq!(*kind, ToolKind::Execute);
                assert_eq!(*status, ToolCallStatus::InProgress);
                assert_eq!(detail.as_deref(), Some("List files"));
            }
            other => panic!("{other:?}"),
        }
        match &mapped[3] {
            AgentEvent::ToolCall {
                title,
                kind,
                locations,
                ..
            } => {
                assert_eq!(title, "Read");
                assert_eq!(*kind, ToolKind::Read);
                assert_eq!(locations, &vec!["/repo/README.md".to_string()]);
            }
            other => panic!("{other:?}"),
        }

        let results = decode_event(
            &json!({
                "type": "user",
                "message": {"role": "user", "content": [
                    {"type": "tool_result", "tool_use_id": "toolu_1", "content": "ok"},
                    {"type": "tool_result", "tool_use_id": "toolu_2", "is_error": true, "content": "nope"}
                ]}
            })
            .to_string(),
        )
        .unwrap();
        let statuses: Vec<_> = map_claude_event(&results)
            .into_iter()
            .map(|event| match event {
                AgentEvent::ToolCallUpdate { id, status, .. } => (id, status),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(
            statuses,
            vec![
                ("toolu_1".to_string(), Some(ToolCallStatus::Completed)),
                ("toolu_2".to_string(), Some(ToolCallStatus::Failed)),
            ]
        );
    }

    #[test]
    fn maps_permission() {
        let mapped = map_claude_event(&Event::CanUseTool(CanUseTool {
            request_id: "req".into(),
            tool_name: "Bash".into(),
            input: json!({"command": "echo hi"}),
            tool_use_id: Some("t1".into()),
            decision_reason: None,
        }));
        match mapped.into_iter().next().unwrap() {
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

    #[test]
    fn a_threads_cost_adds_up_each_turn_across_processes() {
        // Two turns in one process, then a resumed process that starts from zero.
        let (total, seen) = add_turn_cost(0.0, 0.0, 0.002);
        let (total, seen) = add_turn_cost(total, seen, 0.005);
        assert!((total - 0.005).abs() < 1e-9);
        let (total, _) = add_turn_cost(total, 0.0, 0.001);
        assert!((total - 0.006).abs() < 1e-9);
        assert_eq!(seen, 0.005);
    }

    #[test]
    fn maps_usage_from_result() {
        let usage = usage_from_result(&Event::Result {
            session_id: None,
            is_error: false,
            subtype: Some("success".into()),
            result: None,
            used_tokens: Some(120),
            context_size: Some(200_000),
            cost_usd: Some("0.02".into()),
        })
        .unwrap();
        match usage {
            AgentEvent::Usage {
                used_tokens,
                context_size,
                cost_usd,
            } => {
                assert_eq!(used_tokens, 120);
                assert_eq!(context_size, 200_000);
                assert_eq!(cost_usd, Some(0.02));
            }
            other => panic!("{other:?}"),
        }
    }
}
