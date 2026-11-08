//! A thread's event log: everything the thread shows, saved in order and streamed to whoever is
//! watching it. This is the only writer of `thread_events` and the only reader of its rows' kinds
//! and payloads: the webview gets them decoded, each with its time and the conversation it is in.
//!
//! What is saved and what is only streamed:
//! - Saved ([`Transcript::append`]): the user's messages, what the engine said and did (messages,
//!   tool calls and their results, plans, usage, session starts, turn ends), permission requests
//!   waiting on the user, the core's own steps, and a findings batch's live title.
//! - Streamed only ([`Transcript::signal`]): an engine exiting (an engine reaped for sitting idle
//!   exits too, and that isn't the thread moving), a permission approved without asking (a chip on
//!   the step that follows), the tool a permission is waiting on, and the stream falling behind.
//!   A subscriber that lags can read back every saved event; the signals it missed are gone.

pub mod commands;
mod conversations;
mod read;
mod store;
mod upgrade;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::sync::broadcast;

use crate::engines::AgentEvent;
use crate::error::{Error, Result};
use crate::store::Store;
use crate::timeline::TimelineStep;

pub use conversations::ThreadConversations;
use conversations::{Conversations, Counted, is_boundary};
pub use store::ThreadActivityRow;
pub(crate) use store::activity_in;
pub use upgrade::MIGRATION;

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

    /// A session step, which begins a conversation.
    fn begins_conversation(&self) -> bool {
        matches!(self, Self::Step(StepEvent::Step { step, .. }) if is_boundary(step))
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
    /// The conversation it is in; see [`conversations`].
    pub conversation: String,
    pub event: TranscriptEvent,
}

/// A saved event as the timeline reads it.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SavedEvent {
    pub seq: i64,
    /// When it was saved, in Unix milliseconds.
    pub at_ms: i64,
    pub conversation: String,
    pub event: TranscriptEvent,
}

/// A window of a thread's transcript, oldest first. Rows this build can't read are left out (and
/// logged); the seqs bound every row read, so paging carries on past them.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptPage {
    pub events: Vec<SavedEvent>,
    pub oldest_seq: Option<i64>,
    pub newest_seq: Option<i64>,
    pub has_older: bool,
    pub has_newer: bool,
}

/// The thread event log over the store, plus a live stream per watched thread.
#[derive(Clone)]
pub struct Transcript {
    store: Store,
    streams: Arc<Mutex<HashMap<String, broadcast::Sender<StreamedEvent>>>>,
    /// Each streamed thread's current conversation, until its next session step.
    current: Arc<Mutex<HashMap<String, String>>>,
}

impl Transcript {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            streams: Arc::new(Mutex::new(HashMap::new())),
            current: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// One window of a thread's transcript: its newest rows by default, those after `after_seq`
    /// to catch up, or those before `before_seq` to scroll back.
    pub fn window(
        &self,
        thread_id: &str,
        after_seq: Option<i64>,
        before_seq: Option<i64>,
        limit: Option<u32>,
    ) -> Result<TranscriptPage> {
        let page = self
            .store
            .thread_event_page(thread_id, after_seq, before_seq, limit)?;
        let conversations = self.conversations_of(thread_id)?;
        let oldest_seq = page.events.first().map(|row| row.seq);
        let newest_seq = page.events.last().map(|row| row.seq);
        let events = page
            .events
            .into_iter()
            .filter_map(|row| match serde_json::from_str(&row.payload) {
                Ok(event) => Some(SavedEvent {
                    seq: row.seq,
                    at_ms: row.at_ms,
                    conversation: conversations.at(row.seq).to_string(),
                    event,
                }),
                Err(error) => {
                    log::warn!(
                        "thread {thread_id}: skipped unreadable {} row {}: {error}",
                        row.kind,
                        row.seq
                    );
                    None
                }
            })
            .collect();
        Ok(TranscriptPage {
            events,
            oldest_seq,
            newest_seq,
            has_older: page.has_older,
            has_newer: page.has_newer,
        })
    }

    /// The conversation showing now and the earlier ones the thread can go back to.
    pub fn conversations(&self, thread_id: &str) -> Result<ThreadConversations> {
        let steps = self.store.session_steps(thread_id)?;
        let boundaries: std::collections::HashSet<i64> =
            steps.iter().map(|(seq, _)| *seq).collect();
        let conversations = Self::decode_boundaries(thread_id, steps);
        let rows = self.store.message_rows(thread_id)?;
        Ok(conversations::summarise(
            &conversations,
            rows.into_iter().map(|(seq, at_ms, payload)| Counted {
                seq,
                at_ms,
                boundary: boundaries.contains(&seq),
                user_text: payload.and_then(|payload| match serde_json::from_str(&payload) {
                    Ok(TranscriptEvent::UserMessage(UserMessage::MessageChunk {
                        text, ..
                    })) => Some(text),
                    _ => None,
                }),
            }),
        ))
    }

    /// The agent session to resume conversation `id` with; none starts the agent fresh. Errors
    /// for an id the thread doesn't have.
    pub fn session_to_resume(&self, thread_id: &str, id: &str) -> Result<Option<String>> {
        self.conversations_of(thread_id)?.session_of(id)
    }

    fn conversations_of(&self, thread_id: &str) -> Result<Conversations> {
        Ok(Self::decode_boundaries(
            thread_id,
            self.store.session_steps(thread_id)?,
        ))
    }

    fn decode_boundaries(thread_id: &str, steps: Vec<(i64, String)>) -> Conversations {
        Conversations::from_boundaries(steps.into_iter().filter_map(|(seq, payload)| {
            match serde_json::from_str(&payload) {
                Ok(TranscriptEvent::Step(StepEvent::Step { step, .. })) => Some((seq, step)),
                Ok(_) => None,
                Err(error) => {
                    log::warn!(
                        "thread {thread_id}: skipped unreadable session step {seq}: {error}"
                    );
                    None
                }
            }
        }))
    }

    /// The conversation newly streamed events are in.
    fn current_conversation(&self, thread_id: &str) -> String {
        let mut current = self
            .current
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(id) = current.get(thread_id) {
            return id.clone();
        }
        let id = match self.conversations_of(thread_id) {
            Ok(conversations) => conversations.current().to_string(),
            Err(error) => {
                log::warn!("thread {thread_id}: reading its conversations failed: {error}");
                return "first".into();
            }
        };
        current.insert(thread_id.to_string(), id.clone());
        id
    }

    /// Saves the event, then streams it with its `seq`. One that fails to save still streams,
    /// without a `seq`, so the thread on screen doesn't lose it; the error comes back.
    pub fn append(&self, thread_id: &str, event: impl Into<TranscriptEvent>) -> Result<i64> {
        let event = event.into();
        let saved = self.save(thread_id, &event);
        if event.begins_conversation() {
            self.current
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .remove(thread_id);
        }
        self.publish(
            thread_id,
            StreamedEvent {
                seq: saved.as_ref().ok().copied(),
                conversation: self.current_conversation(thread_id),
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
                conversation: self.current_conversation(thread_id),
                event: event.into(),
            },
        );
    }

    /// What a subscriber that fell `missed` events behind is sent instead of them.
    pub fn lagged(&self, thread_id: &str, missed: u64) -> StreamedEvent {
        StreamedEvent {
            seq: None,
            conversation: self.current_conversation(thread_id),
            event: TranscriptEvent::Lagged(StreamLagged::StreamLagged { missed }),
        }
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
    fn a_window_is_decoded_with_times_and_conversations_and_skips_unreadable_rows() {
        let transcript = with_threads(&["t1"]);
        transcript.record_user_message("t1", "first ask").unwrap();
        transcript
            .store
            .append_event("t1", "tool", "{not json")
            .unwrap();
        transcript
            .record_step(
                "t1",
                TimelineStep::SessionStarted {
                    ended_session: Some("s1".into()),
                },
            )
            .unwrap();
        transcript.record_user_message("t1", "second ask").unwrap();

        let page = transcript.window("t1", None, None, None).unwrap();
        let seen: Vec<_> = page
            .events
            .iter()
            .map(|saved| (saved.seq, saved.conversation.as_str()))
            .collect();
        assert_eq!(seen, [(1, "first"), (3, "after-3"), (4, "after-3")]);
        assert_eq!((page.oldest_seq, page.newest_seq), (Some(1), Some(4)));
        assert!(page.events[0].at_ms > 1_700_000_000_000);
        assert_eq!(
            page.events[0].event,
            TranscriptEvent::user_message("first ask")
        );

        let conversations = transcript.conversations("t1").unwrap();
        assert_eq!(conversations.current, "after-3");
        assert_eq!(conversations.current_starts_at, Some(4));
        assert_eq!(conversations.earlier.len(), 1);
        assert_eq!(
            conversations.earlier[0].first_prompt.as_deref(),
            Some("first ask")
        );
        assert_eq!(
            transcript
                .session_to_resume("t1", "first")
                .unwrap()
                .as_deref(),
            Some("s1")
        );
    }

    #[tokio::test]
    async fn streamed_events_carry_the_conversation_a_session_step_begins() {
        let transcript = with_threads(&["t1"]);
        let mut rx = transcript.subscribe("t1");
        transcript.record_user_message("t1", "go").unwrap();
        transcript
            .record_step(
                "t1",
                TimelineStep::SessionStarted {
                    ended_session: None,
                },
            )
            .unwrap();
        transcript.signal("t1", AgentEvent::EngineExited { code: None });

        let conversations: Vec<_> = [
            rx.recv().await.unwrap(),
            rx.recv().await.unwrap(),
            rx.recv().await.unwrap(),
        ]
        .into_iter()
        .map(|streamed| streamed.conversation)
        .collect();
        assert_eq!(conversations, ["first", "after-2", "after-2"]);
    }

    #[test]
    fn a_signal_cannot_be_saved() {
        let transcript = with_threads(&["t1"]);
        let lagged = TranscriptEvent::Lagged(StreamLagged::StreamLagged { missed: 3 });
        assert!(transcript.append("t1", lagged).is_err());
    }
}
