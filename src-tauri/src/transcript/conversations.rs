//! A thread's conversations: the stretches of its transcript that belong together. Starting a new
//! session begins one; resuming switches back to an earlier one, whose later events join it.
//!
//! Ids are `first` for the thread's opening conversation and `after-<seq>` for one begun by the
//! session step saved at `seq`. Resumed-session steps store the id they went back to, so the
//! scheme can't change.

use std::collections::HashMap;

use serde::Serialize;
use specta::Type;

use crate::error::{Error, Result};
use crate::timeline::TimelineStep;

/// The webview gets this as `FIRST_CONVERSATION`.
pub const FIRST_CONVERSATION: &str = "first";

/// The kinds of step that end the conversation before them, as saved. The one list of them:
/// the store finds saved ones by it, streamed events are matched against it, and the webview
/// gets it as `CONVERSATION_STEP_KINDS`. `boundary` reads each of them.
pub const CONVERSATION_STEP_KINDS: [&str; 2] = ["sessionStarted", "sessionResumed"];

pub(super) fn is_boundary(step: &TimelineStep) -> bool {
    serde_json::to_value(step).is_ok_and(|value| {
        value["kind"]
            .as_str()
            .is_some_and(|kind| CONVERSATION_STEP_KINDS.contains(&kind))
    })
}

/// A boundary step's session it ended the conversation before it with, and the earlier
/// conversation it went back to, if it did.
fn boundary(step: TimelineStep) -> Option<(Option<String>, Option<String>)> {
    match step {
        TimelineStep::SessionStarted { ended_session } => Some((ended_session, None)),
        TimelineStep::SessionResumed {
            conversation,
            ended_session,
        } => Some((ended_session, Some(conversation))),
        _ => None,
    }
}

/// Which conversation each saved row is in, from the thread's session steps.
#[derive(Debug, Clone)]
pub(super) struct Conversations {
    /// Where each stretch starts (a session step's seq, 0 for the first) and whose it is.
    stretches: Vec<(i64, String)>,
    /// The agent session each conversation last ended with, to resume it by.
    ended: HashMap<String, String>,
}

impl Conversations {
    /// From the thread's session steps, oldest first.
    pub(super) fn from_boundaries(steps: impl IntoIterator<Item = (i64, TimelineStep)>) -> Self {
        let mut stretches = vec![(0, FIRST_CONVERSATION.to_string())];
        let mut ended = HashMap::new();
        for (seq, step) in steps {
            let current = stretches
                .last()
                .map(|(_, id)| id.clone())
                .unwrap_or_default();
            let Some((ended_session, target)) = boundary(step) else {
                continue;
            };
            if let Some(session) = ended_session {
                ended.insert(current, session);
            }
            let next = target
                .filter(|id| stretches.iter().any(|(_, known)| known == id))
                .unwrap_or_else(|| format!("after-{seq}"));
            stretches.push((seq, next));
        }
        Self { stretches, ended }
    }

    /// The conversation the row at `seq` is in; a session step is in the one it begins.
    pub(super) fn at(&self, seq: i64) -> &str {
        let index = self.stretches.partition_point(|(start, _)| *start <= seq);
        &self.stretches[index.saturating_sub(1)].1
    }

    pub(super) fn current(&self) -> &str {
        &self.stretches[self.stretches.len() - 1].1
    }

    /// The session to resume `id` with: the one it last ended with, if one was recorded.
    pub(super) fn session_of(&self, id: &str) -> Result<Option<String>> {
        if !self.stretches.iter().any(|(_, known)| known == id) {
            return Err(Error::Invalid(format!("unknown conversation {id}")));
        }
        Ok(self.ended.get(id).cloned())
    }
}

/// A conversation the thread can go back to.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSummary {
    pub id: String,
    /// The first thing you asked, to name it by.
    pub first_prompt: Option<String>,
    pub prompts: u32,
    pub last_at_ms: i64,
}

/// The conversation showing now, and the earlier ones that can be resumed.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ThreadConversations {
    pub current: String,
    /// The current conversation's oldest saved row, if it has one; a resumed conversation's
    /// start can be far older than the newest page.
    pub current_starts_at: Option<i64>,
    /// Those with something you asked, newest first.
    pub earlier: Vec<ConversationSummary>,
}

/// One saved row as summaries count it: its seq, time, and your message's text if it is one.
pub(super) struct Counted {
    pub seq: i64,
    pub at_ms: i64,
    pub user_text: Option<String>,
    pub boundary: bool,
}

pub(super) fn summarise(
    conversations: &Conversations,
    rows: impl IntoIterator<Item = Counted>,
) -> ThreadConversations {
    let current = conversations.current().to_string();
    let mut current_starts_at = None;
    let mut summaries: Vec<ConversationSummary> = Vec::new();
    for row in rows {
        if row.boundary {
            continue;
        }
        let id = conversations.at(row.seq);
        if id == current {
            current_starts_at.get_or_insert(row.seq);
        }
        let summary = match summaries.iter_mut().position(|summary| summary.id == id) {
            Some(index) => &mut summaries[index],
            None => {
                summaries.push(ConversationSummary {
                    id: id.to_string(),
                    first_prompt: None,
                    prompts: 0,
                    last_at_ms: row.at_ms,
                });
                summaries.last_mut().expect("just pushed")
            }
        };
        summary.last_at_ms = row.at_ms;
        if let Some(text) = row.user_text {
            summary.prompts += 1;
            if summary.first_prompt.is_none() {
                let text = text.trim();
                summary.first_prompt = (!text.is_empty()).then(|| text.to_string());
            }
        }
    }
    let mut earlier: Vec<_> = summaries
        .into_iter()
        .filter(|summary| summary.id != current && summary.prompts > 0)
        .collect();
    earlier.sort_by_key(|summary| std::cmp::Reverse(summary.last_at_ms));
    ThreadConversations {
        current,
        current_starts_at,
        earlier,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn started(ended: Option<&str>) -> TimelineStep {
        TimelineStep::SessionStarted {
            ended_session: ended.map(Into::into),
        }
    }

    fn resumed(to: &str, ended: Option<&str>) -> TimelineStep {
        TimelineStep::SessionResumed {
            conversation: to.into(),
            ended_session: ended.map(Into::into),
        }
    }

    /// Every kind the store and the stream treat as a boundary is one `from_boundaries` reads.
    #[test]
    fn each_conversation_step_kind_is_read_as_a_boundary() {
        for kind in CONVERSATION_STEP_KINDS {
            let step: TimelineStep = serde_json::from_value(serde_json::json!({
                "kind": kind, "conversation": "first", "ended_session": null
            }))
            .unwrap();
            assert!(is_boundary(&step), "{kind}");
            assert!(boundary(step).is_some(), "{kind}");
        }
        assert!(!is_boundary(&TimelineStep::AgentStopped));
    }

    #[test]
    fn a_thread_without_session_steps_is_one_conversation() {
        let conversations = Conversations::from_boundaries([]);
        assert_eq!(conversations.current(), "first");
        assert_eq!(conversations.at(7), "first");
        assert_eq!(conversations.session_of("first").unwrap(), None);
    }

    #[test]
    fn new_sessions_begin_conversations_and_resuming_joins_an_earlier_one() {
        let conversations = Conversations::from_boundaries([
            (3, started(Some("s1"))),
            (6, resumed("first", Some("s2"))),
        ]);
        assert_eq!(conversations.at(2), "first");
        assert_eq!(conversations.at(3), "after-3");
        assert_eq!(conversations.at(5), "after-3");
        assert_eq!(conversations.at(9), "first");
        assert_eq!(conversations.current(), "first");
        assert_eq!(
            conversations.session_of("first").unwrap().as_deref(),
            Some("s1")
        );
        assert_eq!(
            conversations.session_of("after-3").unwrap().as_deref(),
            Some("s2")
        );
        assert!(conversations.session_of("after-4").is_err());
    }

    #[test]
    fn resuming_an_unknown_conversation_begins_a_new_one() {
        let conversations = Conversations::from_boundaries([(4, resumed("gone", None))]);
        assert_eq!(conversations.current(), "after-4");
    }

    #[test]
    fn summaries_name_earlier_conversations_by_their_first_prompt_newest_first() {
        let conversations =
            Conversations::from_boundaries([(3, started(None)), (6, started(None))]);
        let row = |seq, at_ms, text: Option<&str>| Counted {
            seq,
            at_ms,
            user_text: text.map(Into::into),
            boundary: false,
        };
        let summary = summarise(
            &conversations,
            [
                row(1, 10, Some("  ")),
                row(2, 20, Some(" fix it ")),
                Counted {
                    seq: 3,
                    at_ms: 30,
                    user_text: None,
                    boundary: true,
                },
                row(4, 40, Some("then this")),
                row(5, 50, None),
                Counted {
                    seq: 6,
                    at_ms: 60,
                    user_text: None,
                    boundary: true,
                },
            ],
        );
        assert_eq!(summary.current, "after-6");
        assert_eq!(summary.current_starts_at, None);
        assert_eq!(
            summary.earlier,
            [
                ConversationSummary {
                    id: "after-3".into(),
                    first_prompt: Some("then this".into()),
                    prompts: 1,
                    last_at_ms: 50,
                },
                ConversationSummary {
                    id: "first".into(),
                    first_prompt: Some("fix it".into()),
                    prompts: 2,
                    last_at_ms: 20,
                },
            ]
        );
    }
}
