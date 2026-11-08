//! A thread's event log: everything the thread shows, saved in order and streamed to whoever is
//! watching it. This is the only writer of `thread_events` and the only reader of its rows' kinds
//! and payloads.
//!
//! What is saved and what is only streamed:
//! - Saved ([`Transcript::append`]): the user's messages, what the engine said and did (messages,
//!   tool calls and their results, plans, usage, session starts, turn ends), permission requests
//!   waiting on the user, the core's own steps, and a findings batch's live title.
//! - Streamed only ([`Transcript::signal`]): an engine exiting (an engine reaped for sitting idle
//!   exits too, and that isn't the thread moving), a permission approved without asking (a chip on
//!   the step that follows), the tool a permission is waiting on, and the stream falling behind.
//!   A subscriber that lags can read back every saved event; the signals it missed are gone.

mod read;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::sync::broadcast;

use crate::engines::AgentEvent;
use crate::error::{Error, Result};
use crate::store::Store;
use crate::timeline::TimelineStep;

/// Events a subscriber can fall behind by before it's told to read the store instead.
const STREAM_CAP: usize = 256;

/// One row of a thread's transcript. The wire form is the stored payload: each variant carries its
/// own `type`, so they share one union in the UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(untagged)]
pub enum TranscriptEvent {
    Step(StepEvent),
    UserMessage(UserMessage),
    Briefing(Briefing),
    Lagged(StreamLagged),
    Agent(AgentEvent),
}

/// Something the core did in the thread rather than the agent; see [`TimelineStep`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StepEvent {
    Step {
        id: String,
        /// Display text only; the kind in `step` decides how the step looks.
        title: String,
        detail: Option<String>,
        step: TimelineStep,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum UserRole {
    User,
}

/// What the user sent, as the thread shows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum UserMessage {
    MessageChunk { role: UserRole, text: String },
}

/// A prompt the thread was started with that the timeline doesn't show: a review's instructions,
/// which its checked-out-PR step stands for. Readers of what the user asked still see it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Briefing {
    Briefing { role: UserRole, text: String },
}

/// Never saved: the subscriber fell behind the stream and dropped `missed` events, which it can
/// read back from the store.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StreamLagged {
    StreamLagged { missed: u64 },
}

impl TranscriptEvent {
    pub fn user_message(text: impl Into<String>) -> Self {
        Self::UserMessage(UserMessage::MessageChunk {
            role: UserRole::User,
            text: text.into(),
        })
    }

    pub fn briefing(text: impl Into<String>) -> Self {
        Self::Briefing(Briefing::Briefing {
            role: UserRole::User,
            text: text.into(),
        })
    }

    pub fn step(step: TimelineStep) -> Self {
        Self::Step(StepEvent::Step {
            id: step.id(),
            title: step.title(),
            detail: step.detail(),
            step,
        })
    }

    /// The `kind` column the event is saved under.
    fn kind(&self) -> Option<&'static str> {
        Some(match self {
            Self::Step(_) => TOOL,
            Self::UserMessage(_) | Self::Briefing(_) => MESSAGE,
            Self::Lagged(_) => return None,
            Self::Agent(event) => match event {
                AgentEvent::SessionStarted { .. } => "session",
                AgentEvent::MessageChunk { .. } => MESSAGE,
                AgentEvent::ToolCall { .. } => TOOL,
                AgentEvent::Plan { .. } => "plan",
                AgentEvent::Permission { .. } => "permission",
                AgentEvent::ToolCallUpdate { .. } => "tool_update",
                AgentEvent::CurrentTool { .. } => "current_tool",
                AgentEvent::Usage { .. } => "usage",
                AgentEvent::TurnEnd { .. } => TURN_END,
                AgentEvent::EngineExited { .. } => EXIT,
            },
        })
    }
}

impl From<AgentEvent> for TranscriptEvent {
    fn from(event: AgentEvent) -> Self {
        Self::Agent(event)
    }
}

const MESSAGE: &str = "message";
const TOOL: &str = "tool";
const TURN_END: &str = "turn_end";
const EXIT: &str = "exit";

/// An event on its way to a subscriber, with where the store saved it: `None` for a signal, or an
/// event that failed to save.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct StreamedEvent {
    pub seq: Option<i64>,
    pub event: TranscriptEvent,
}

/// The thread event log over the store, plus a live stream per watched thread.
#[derive(Clone)]
pub struct Transcript {
    store: Store,
    streams: Arc<Mutex<HashMap<String, broadcast::Sender<StreamedEvent>>>>,
}

impl Transcript {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            streams: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Saves the event, then streams it with its `seq`. One that fails to save still streams,
    /// without a `seq`, so the thread on screen doesn't lose it; the error comes back.
    pub fn append(&self, thread_id: &str, event: impl Into<TranscriptEvent>) -> Result<i64> {
        let event = event.into();
        let saved = self.save(thread_id, &event);
        self.publish(
            thread_id,
            StreamedEvent {
                seq: saved.as_ref().ok().copied(),
                event,
            },
        );
        saved
    }

    /// Streams an event without saving it; see the module docs for which these are.
    pub fn signal(&self, thread_id: &str, event: AgentEvent) {
        self.publish(
            thread_id,
            StreamedEvent {
                seq: None,
                event: event.into(),
            },
        );
    }

    pub fn record_step(&self, thread_id: &str, step: TimelineStep) -> Result<i64> {
        self.append(thread_id, TranscriptEvent::step(step))
    }

    pub fn record_user_message(&self, thread_id: &str, text: &str) -> Result<i64> {
        self.append(thread_id, TranscriptEvent::user_message(text))
    }

    /// Everything appended or signalled to the thread from now on.
    pub fn subscribe(&self, thread_id: &str) -> broadcast::Receiver<StreamedEvent> {
        let mut streams = self
            .streams
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        streams
            .entry(thread_id.to_string())
            .or_insert_with(|| broadcast::channel(STREAM_CAP).0)
            .subscribe()
    }

    fn save(&self, thread_id: &str, event: &TranscriptEvent) -> Result<i64> {
        let kind = event
            .kind()
            .ok_or_else(|| Error::Store("a stream signal can't be saved".into()))?;
        let payload =
            serde_json::to_string(event).map_err(|error| Error::Store(error.to_string()))?;
        self.store.append_event(thread_id, kind, &payload)
    }

    fn publish(&self, thread_id: &str, event: StreamedEvent) {
        let mut streams = self
            .streams
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some(stream) = streams.get(thread_id) else {
            return;
        };
        if stream.send(event).is_err() {
            // Nobody is watching any more.
            streams.remove(thread_id);
        }
    }
}

/// A transcript over an in-memory store holding `threads`.
#[cfg(test)]
pub fn with_threads(threads: &[&str]) -> Transcript {
    let store = Store::in_memory().unwrap();
    for id in threads {
        store
            .upsert_thread(&crate::store::types::ThreadRow::new(
                *id,
                "w1",
                crate::model::ThreadRole::Lead,
                "claude",
            ))
            .unwrap();
    }
    Transcript::new(store)
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;
    use crate::engines::MessageRole;

    fn rows(transcript: &Transcript) -> Vec<(String, Value)> {
        transcript
            .store
            .thread_events_after("t1", 0, 100)
            .unwrap()
            .into_iter()
            .map(|(_, kind, payload)| (kind, serde_json::from_str(&payload).unwrap()))
            .collect()
    }

    #[test]
    fn a_step_is_saved_as_a_tool_row_with_its_kind_and_display_text() {
        let transcript = with_threads(&["t1"]);
        transcript
            .record_step(
                "t1",
                TimelineStep::PrOpened {
                    number: 12,
                    branch: "feat/a".into(),
                    base: "main".into(),
                },
            )
            .unwrap();
        assert_eq!(
            rows(&transcript),
            [(
                "tool".to_string(),
                json!({
                    "type": "step",
                    "id": "pr-open-12",
                    "title": "Opened PR #12",
                    "detail": "feat/a → main",
                    "step": { "kind": "prOpened", "number": 12, "branch": "feat/a", "base": "main" },
                })
            )]
        );
    }

    #[test]
    fn every_kind_of_row_reads_back_as_the_event_written() {
        let events = [
            TranscriptEvent::step(TimelineStep::BranchSwitched {
                branch: "feat/b".into(),
                change: crate::timeline::BranchChange::AddedToStack {
                    on_top_of: "feat/a".into(),
                },
            }),
            TranscriptEvent::user_message("hi"),
            TranscriptEvent::briefing("review this"),
            AgentEvent::MessageChunk {
                role: MessageRole::Agent,
                text: "hello".into(),
            }
            .into(),
        ];
        for event in events {
            let text = serde_json::to_string(&event).unwrap();
            assert_eq!(
                serde_json::from_str::<TranscriptEvent>(&text).unwrap(),
                event
            );
        }
        let user = serde_json::to_value(TranscriptEvent::user_message("hi")).unwrap();
        assert_eq!(
            user,
            json!({ "type": "messageChunk", "role": "user", "text": "hi" })
        );
    }

    #[tokio::test]
    async fn subscribers_get_saved_events_with_their_seq_and_signals_without() {
        let transcript = with_threads(&["t1"]);
        let mut rx = transcript.subscribe("t1");
        transcript.record_user_message("t1", "go").unwrap();
        transcript.signal("t1", AgentEvent::EngineExited { code: None });

        let first = rx.recv().await.unwrap();
        assert_eq!(first.seq, Some(1));
        assert_eq!(first.event, TranscriptEvent::user_message("go"));
        let second = rx.recv().await.unwrap();
        assert_eq!(second.seq, None);
        assert_eq!(rows(&transcript).len(), 1, "a signal isn't saved");
    }

    #[test]
    fn a_signal_cannot_be_saved() {
        let transcript = with_threads(&["t1"]);
        let lagged = TranscriptEvent::Lagged(StreamLagged::StreamLagged { missed: 3 });
        assert!(transcript.append("t1", lagged).is_err());
    }
}
