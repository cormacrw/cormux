//! Typed reads of a thread's transcript, for everything that isn't the timeline.

use crate::engines::{AgentEvent, MessageRole};
use crate::error::Result;

use super::{MESSAGE, StepEvent, TOOL, Transcript, TranscriptEvent};

/// Who an entry is from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speaker {
    User,
    Agent,
    Thought,
    Tool,
}

impl Speaker {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Agent => "agent",
            Self::Thought => "thought",
            Self::Tool => "tool",
        }
    }
}

/// One message with its streamed chunks joined, or one tool call or step by its title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub speaker: Speaker,
    pub text: String,
}

/// A page of entries, and the `since` that continues after it (`None` at the end).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub entries: Vec<Entry>,
    pub next: Option<i64>,
}

/// Who said what in a saved row, if it is a message, a tool call or a step.
fn said(kind: &str, payload: &str) -> Option<(Speaker, String)> {
    if kind != MESSAGE && kind != TOOL {
        return None;
    }
    let event = serde_json::from_str::<TranscriptEvent>(payload).ok()?;
    Some(match event {
        TranscriptEvent::UserMessage(super::UserMessage::MessageChunk { text, .. })
        | TranscriptEvent::Briefing(super::Briefing::Briefing { text, .. }) => {
            (Speaker::User, text)
        }
        TranscriptEvent::Agent(AgentEvent::MessageChunk { role, text }) => match role {
            MessageRole::Agent => (Speaker::Agent, text),
            MessageRole::Thought => (Speaker::Thought, text),
        },
        TranscriptEvent::Agent(AgentEvent::ToolCall { title, .. })
        | TranscriptEvent::Step(StepEvent::Step { title, .. }) => (Speaker::Tool, title),
        _ => return None,
    })
}

/// Consecutive chunks from one speaker are one message; each tool call stays its own entry.
fn join(said: impl IntoIterator<Item = (Speaker, String)>) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    for (speaker, text) in said {
        match entries.last_mut() {
            Some(last) if last.speaker == speaker && speaker != Speaker::Tool => {
                last.text.push_str(&text);
            }
            _ => entries.push(Entry { speaker, text }),
        }
    }
    entries
}

impl Transcript {
    /// Everything the agent said since the user's last message.
    pub fn last_agent_reply(&self, thread_id: &str) -> Result<String> {
        let mut since_user = Vec::new();
        self.store
            .visit_events(thread_id, Some(MESSAGE), true, |_, kind, payload| {
                match said(kind, payload) {
                    Some((Speaker::User, _)) => return false,
                    Some(said) => since_user.push(said),
                    None => {}
                }
                true
            })?;
        since_user.reverse();
        Ok(join(since_user)
            .into_iter()
            .filter(|entry| entry.speaker == Speaker::Agent)
            .map(|entry| entry.text)
            .collect())
    }

    /// Where the thread's conversation last moved, so a turn is only reported once.
    pub fn last_message_seq(&self, thread_id: &str) -> Result<i64> {
        self.store.last_seq_of_kind(thread_id, MESSAGE)
    }

    /// Entries after `since`, reading at most `max_events` rows and stopping once `max_chars` of
    /// text are in.
    pub fn page(
        &self,
        thread_id: &str,
        since: i64,
        max_events: i64,
        max_chars: usize,
    ) -> Result<Page> {
        let rows = self
            .store
            .thread_events_after(thread_id, since, max_events)?;
        let full_page = rows.len() as i64 == max_events;
        let mut chars = 0;
        let mut last_seq = since;
        let mut cut_short = false;
        let mut kept = Vec::new();
        for (seq, kind, payload) in rows {
            if chars >= max_chars {
                cut_short = true;
                break;
            }
            last_seq = seq;
            if let Some((speaker, text)) = said(&kind, &payload) {
                chars += text.chars().count();
                kept.push((speaker, text));
            }
        }
        Ok(Page {
            entries: join(kept),
            next: (full_page || cut_short).then_some(last_seq),
        })
    }

    /// The first message in the thread, which is what it was started to do.
    pub fn first_message(&self, thread_id: &str) -> Result<Option<String>> {
        let mut first = None;
        self.store
            .visit_events(thread_id, Some(MESSAGE), false, |_, kind, payload| {
                first = said(kind, payload).map(|(_, text)| text);
                false
            })?;
        Ok(first)
    }

    /// The first non-blank thing the user asked for in the thread.
    pub fn first_user_goal(&self, thread_id: &str) -> Result<String> {
        let mut goal = String::new();
        self.store
            .visit_events(thread_id, Some(MESSAGE), false, |_, kind, payload| {
                if let Some((Speaker::User, text)) = said(kind, payload)
                    && !text.trim().is_empty()
                {
                    goal = text.trim().to_string();
                    return false;
                }
                true
            })?;
        Ok(goal)
    }

    /// The newest `limit` rows as `kind: payload` lines, oldest first, for a model to read.
    pub fn raw_tail(&self, thread_id: &str, limit: usize) -> Result<String> {
        let mut lines = Vec::new();
        self.store
            .visit_events(thread_id, None, true, |_, kind, payload| {
                lines.push(format!("{kind}: {payload}"));
                lines.len() < limit
            })?;
        lines.reverse();
        Ok(lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;
    use crate::timeline::TimelineStep;

    fn agent(text: &str) -> AgentEvent {
        AgentEvent::MessageChunk {
            role: MessageRole::Agent,
            text: text.into(),
        }
    }

    fn thought(text: &str) -> AgentEvent {
        AgentEvent::MessageChunk {
            role: MessageRole::Thought,
            text: text.into(),
        }
    }

    fn transcript() -> Transcript {
        super::super::with_threads(&["t1"])
    }

    #[test]
    fn the_last_reply_is_every_agent_chunk_since_the_last_user_message() {
        let transcript = transcript();
        transcript.append("t1", agent("old")).unwrap();
        transcript.record_user_message("t1", "go").unwrap();
        transcript.append("t1", thought("hmm")).unwrap();
        transcript.append("t1", agent("Looks ")).unwrap();
        transcript
            .record_step("t1", TimelineStep::AgentPaused)
            .unwrap();
        transcript.append("t1", agent("good.")).unwrap();
        assert_eq!(transcript.last_agent_reply("t1").unwrap(), "Looks good.");
        assert_eq!(transcript.last_agent_reply("none").unwrap(), "");
    }

    #[test]
    fn a_page_joins_chunks_lists_tools_and_says_where_to_continue() {
        let transcript = transcript();
        transcript.record_user_message("t1", "go").unwrap();
        transcript.append("t1", agent("a")).unwrap();
        transcript.append("t1", agent("b")).unwrap();
        transcript
            .record_step("t1", TimelineStep::AgentPaused)
            .unwrap();
        transcript
            .record_step("t1", TimelineStep::AgentResumed)
            .unwrap();
        transcript.append("t1", agent("c")).unwrap();

        let page = transcript.page("t1", 0, 100, 10_000).unwrap();
        let shown: Vec<(&str, &str)> = page
            .entries
            .iter()
            .map(|entry| (entry.speaker.as_str(), entry.text.as_str()))
            .collect();
        assert_eq!(
            shown,
            [
                ("user", "go"),
                ("agent", "ab"),
                ("tool", "You paused the agent"),
                ("tool", "You resumed the agent"),
                ("agent", "c"),
            ]
        );
        assert_eq!(page.next, None);
        assert_eq!(transcript.page("t1", 0, 2, 10_000).unwrap().next, Some(2));
        assert_eq!(transcript.page("t1", 0, 100, 1).unwrap().next, Some(1));
    }

    #[test]
    fn first_message_and_goal_count_a_briefing_as_the_users() {
        let transcript = transcript();
        transcript
            .append("t1", TranscriptEvent::briefing("Review PR #4"))
            .unwrap();
        transcript.record_user_message("t1", "  ").unwrap();
        assert_eq!(
            transcript.first_message("t1").unwrap().as_deref(),
            Some("Review PR #4")
        );
        assert_eq!(transcript.first_user_goal("t1").unwrap(), "Review PR #4");
        assert_eq!(transcript.last_message_seq("t1").unwrap(), 2);
        assert_eq!(transcript.last_message_seq("none").unwrap(), 0);
    }

    #[test]
    fn activity_is_the_last_user_message_turn_end_or_exit() {
        use crate::model::{ThreadStatus, WorkspaceLifecycle};
        let store = Store::in_memory().unwrap();
        store.seed_workspace("w1", WorkspaceLifecycle::Idle);
        for id in ["quiet", "asked", "ended", "exited"] {
            store.seed_thread(id, "w1", ThreadStatus::Idle);
        }
        let transcript = Transcript::new(store.clone());
        transcript.append("quiet", agent("hi")).unwrap();
        transcript
            .record_step("quiet", TimelineStep::AgentPaused)
            .unwrap();
        transcript.record_user_message("asked", "go").unwrap();
        transcript
            .append(
                "ended",
                AgentEvent::TurnEnd {
                    stop_reason: crate::model::StopReason::EndTurn,
                    error: None,
                },
            )
            .unwrap();
        transcript
            .append("exited", AgentEvent::EngineExited { code: None })
            .unwrap();

        let mut moved: Vec<String> = store
            .with_read(super::super::activity_in)
            .unwrap()
            .into_iter()
            .map(|row| row.thread_id)
            .collect();
        moved.sort();
        assert_eq!(moved, ["asked", "ended", "exited"]);
    }

    /// Mid-turn, the lookup must not walk back through a thread's streamed chunks.
    #[test]
    fn the_activity_lookup_uses_its_partial_index() {
        let plan = Store::in_memory().unwrap().activity_query_plan().unwrap();
        assert!(
            plan.contains("thread_events_activity"),
            "activity lookup no longer uses its index: {plan}"
        );
    }
}
