use uuid::Uuid;

use crate::engines::{AgentEvent, MessageRole, ToolCallStatus, ToolKind};
use crate::error::{Error, Result};
use crate::store::Store;

pub fn persist_user_message(store: &Store, thread_id: &str, text: &str) -> Result<()> {
    let event = AgentEvent::MessageChunk {
        role: MessageRole::User,
        text: text.to_string(),
    };
    persist_agent_event(store, thread_id, &event)
}

pub fn persist_control_step(store: &Store, thread_id: &str, title: &str) -> Result<()> {
    let event = AgentEvent::ToolCall {
        id: format!("control-{}", Uuid::new_v4()),
        title: title.to_string(),
        name: None,
        kind: ToolKind::Other,
        status: ToolCallStatus::Completed,
        locations: Vec::new(),
        detail: None,
    };
    persist_agent_event(store, thread_id, &event)
}

fn persist_agent_event(store: &Store, thread_id: &str, event: &AgentEvent) -> Result<()> {
    let kind = match event {
        AgentEvent::MessageChunk { .. } => "message",
        AgentEvent::ToolCall { .. } => "tool",
        _ => "message",
    };
    let payload = serde_json::to_string(event).map_err(|error| Error::Store(error.to_string()))?;
    store.append_event(thread_id, kind, &payload)?;
    Ok(())
}
