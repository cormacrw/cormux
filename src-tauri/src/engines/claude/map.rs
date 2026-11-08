use serde_json::Value;

use super::protocol::{AssistantBlock, Event};
use super::tools::{TODO_TOOL, tool_use};
use crate::engines::events::{AgentEvent, MessageRole, PlanStep, PlanStepStatus, ToolCallStatus};
use crate::model::StopReason;

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
                status: Some(if result.is_error {
                    ToolCallStatus::Failed
                } else {
                    ToolCallStatus::Completed
                }),
                locations: Vec::new(),
                output: result.output.clone(),
            })
            .collect(),
        Event::TaskEnded {
            tool_use_id,
            failed,
        } => vec![AgentEvent::ToolCallUpdate {
            id: tool_use_id.clone(),
            title: None,
            status: Some(if *failed {
                ToolCallStatus::Failed
            } else {
                ToolCallStatus::Completed
            }),
            locations: Vec::new(),
            output: None,
        }],
        Event::Result {
            is_error,
            subtype,
            result,
            ..
        } => vec![AgentEvent::TurnEnd {
            stop_reason: if *is_error {
                StopReason::failed(subtype.clone())
            } else {
                StopReason::EndTurn
            },
            error: if *is_error { result.clone() } else { None },
        }],
        Event::CanUseTool(_)
        | Event::McpMessage { .. }
        | Event::ControlResponse { .. }
        | Event::BackgroundAgents { .. }
        | Event::PromptTaken
        | Event::Other { .. } => Vec::new(),
    }
}

fn map_block(block: &AssistantBlock, parent_id: Option<&str>) -> Option<AgentEvent> {
    // A subagent's words and todo list are its own: the lead gets them back as the Agent
    // call's result, so only its tool calls show, nested under that call.
    if parent_id.is_some()
        && !matches!(block, AssistantBlock::ToolUse { name, .. } if name != TODO_TOOL)
    {
        return None;
    }
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
        AssistantBlock::ToolUse { input, name, .. } if name == TODO_TOOL => Some(todo_plan(input)),
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
                    status: match field("status") {
                        "in_progress" => PlanStepStatus::InProgress,
                        "completed" => PlanStepStatus::Completed,
                        _ => PlanStepStatus::Pending,
                    },
                }
            })
            .collect(),
    }
}

/// A tool Claude started. Tools it runs without asking never reach `can_use_tool`, so this
/// is the only place they show up; a later tool result marks them done.
fn tool_call(id: &str, name: &str, input: &Value, parent_id: Option<&str>) -> AgentEvent {
    let tool = tool_use(name, input);
    AgentEvent::ToolCall {
        parent_id: parent_id.map(str::to_string),
        id: id.to_string(),
        title: tool.title,
        role: tool.role,
        status: ToolCallStatus::InProgress,
        locations: tool.locations,
        detail: tool.detail,
        hidden: tool.hidden,
    }
}

/// A turn's token count and context size, and its process's running cost, from its `result`.
pub fn usage_from_result(event: &Event) -> Option<(u64, u64, Option<f64>)> {
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
    Some((
        used,
        // 0 is "not reported": an unknown window shouldn't read as a full context.
        context_size.unwrap_or(0),
        *cost_usd,
    ))
}

/// Claude reports cost as a running total for its process, which starts over when a session is
/// resumed in a new one. Given the process total last seen and the new one, returns what the
/// turn cost and the process total to remember.
pub fn turn_cost(process_seen: f64, process_total: f64) -> (f64, f64) {
    ((process_total - process_seen).max(0.0), process_total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::ToolRole;
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
                    status: PlanStepStatus::InProgress,
                }],
            })
        );
    }

    #[test]
    fn plan_steps_saved_before_the_status_was_typed_still_read() {
        let saved: PlanStep =
            serde_json::from_value(json!({ "content": "a", "status": "in_progress" })).unwrap();
        assert_eq!(saved.status, PlanStepStatus::InProgress);
        let unknown: PlanStep =
            serde_json::from_value(json!({ "content": "a", "status": "blocked" })).unwrap();
        assert_eq!(unknown.status, PlanStepStatus::Pending);
        assert_eq!(
            serde_json::to_value(PlanStepStatus::InProgress).unwrap(),
            json!("inProgress")
        );
    }

    #[test]
    fn keeps_a_subagents_words_and_todos_out_of_the_thread() {
        use super::super::protocol::decode_event;
        let assistant = decode_event(
            &json!({
                "type": "assistant",
                "parent_tool_use_id": "toolu_agent",
                "message": {"content": [
                    {"type": "thinking", "thinking": "Where is auth?"},
                    {"type": "text", "text": "Found it in auth.ts."},
                    {"type": "tool_use", "id": "toolu_todo", "name": "TodoWrite",
                     "input": {"todos": [{"content": "Read", "status": "pending"}]}},
                    {"type": "tool_use", "id": "toolu_read", "name": "Read",
                     "input": {"file_path": "/repo/auth.ts"}}
                ]}
            })
            .to_string(),
        )
        .unwrap();
        let mapped = map_claude_event(&assistant);
        assert_eq!(mapped.len(), 1);
        assert!(matches!(
            &mapped[0],
            AgentEvent::ToolCall { id, parent_id: Some(parent), .. }
                if id == "toolu_read" && parent == "toolu_agent"
        ));
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
                role,
                status,
                detail,
                ..
            } => {
                assert_eq!(id, "toolu_1");
                assert_eq!(title, "ls -la");
                assert!(matches!(role, ToolRole::Command { command, .. } if command == "ls -la"));
                assert_eq!(*status, ToolCallStatus::InProgress);
                assert_eq!(detail.as_deref(), Some("List files"));
            }
            other => panic!("{other:?}"),
        }
        match &mapped[3] {
            AgentEvent::ToolCall {
                title,
                role,
                locations,
                ..
            } => {
                assert_eq!(title, "Read");
                assert_eq!(*role, ToolRole::Read);
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
    fn each_turn_costs_what_its_process_total_moved_by() {
        // Two turns in one process, then a resumed process that starts from zero.
        let (first, seen) = turn_cost(0.0, 0.002);
        let (second, seen) = turn_cost(seen, 0.005);
        assert!((first + second - 0.005).abs() < 1e-9);
        assert_eq!(seen, 0.005);
        let (resumed, _) = turn_cost(0.0, 0.001);
        assert!((resumed - 0.001).abs() < 1e-9);
    }

    #[test]
    fn a_result_without_usage_reports_none_and_an_unknown_window_reads_as_zero() {
        let result = |used_tokens, context_size| Event::Result {
            session_id: None,
            is_error: false,
            subtype: Some("success".into()),
            result: None,
            used_tokens,
            context_size,
            cost_usd: None,
        };
        assert_eq!(usage_from_result(&result(None, Some(200_000))), None);
        assert_eq!(
            usage_from_result(&result(Some(120), None)),
            Some((120, 0, None))
        );
    }
}
