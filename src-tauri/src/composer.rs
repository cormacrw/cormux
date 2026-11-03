use crate::engines::{AgentEvent, MessageRole};
use crate::error::{Error, Result};
use crate::store::Store;

pub fn persist_user_message(store: &Store, thread_id: &str, text: &str) -> Result<()> {
    let event = AgentEvent::MessageChunk {
        role: MessageRole::User,
        text: text.to_string(),
    };
    persist_agent_event(store, thread_id, &event)
}

pub fn persist_agent_event(store: &Store, thread_id: &str, event: &AgentEvent) -> Result<()> {
    let Some(kind) = event.store_kind() else {
        return Ok(());
    };
    let payload = serde_json::to_string(event).map_err(|error| Error::Store(error.to_string()))?;
    store.append_event(thread_id, kind, &payload)?;
    Ok(())
}
