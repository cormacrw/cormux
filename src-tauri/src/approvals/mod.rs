//! Approvals waiting on the user, from the agent asking to the user answering: the stored row
//! (what the card shows) and the engine's waiter change together, so a card never reads as
//! answered while its agent still waits, or as waiting once its agent has stopped.

pub mod commands;
mod payload;
mod store;

use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex, MutexGuard, PoisonError};

use tokio::sync::oneshot;

#[cfg(test)]
pub use payload::QUESTION_TOOL;
pub use store::ApprovalRow;
pub use store::on_restart;
pub(crate) use store::visible as visible_in;

use crate::bus::{Bus, DomainEvent};
use crate::engines::PermissionRequest;
use crate::error::{Error, Result};
use crate::model::ApprovalStatus;
use crate::settings::Settings;
use crate::store::Store;
use crate::transcript::{HostEvent, Transcript};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ApprovalPolicy {
    run_everything: bool,
}

impl From<&Settings> for ApprovalPolicy {
    fn from(settings: &Settings) -> Self {
        Self {
            run_everything: settings.run_everything,
        }
    }
}

impl ApprovalPolicy {
    /// Only the user can answer a question, so no setting lets one through.
    fn asks_user(self, request: &PermissionRequest) -> bool {
        !request.questions.is_empty() || !self.run_everything
    }
}

/// What the engine is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalDecision {
    Approved,
    Denied {
        message: String,
    },
    /// Answers to the agent's questions, keyed by question text.
    Answered {
        answers: BTreeMap<String, String>,
    },
}

/// What the user said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    Approve,
    Deny { reason: Option<String> },
    Answers(BTreeMap<String, String>),
}

pub struct ApprovalBroker {
    store: Store,
    transcript: Transcript,
    bus: Bus,
    /// Keyed by approval id. Held while a row changes, so the two never disagree.
    waiting: Mutex<HashMap<String, Waiter>>,
}

struct Waiter {
    thread_id: String,
    /// The engine launch that asked; see `ThreadRuntime::is_current`.
    engine: u64,
    decided: oneshot::Sender<ApprovalDecision>,
}

impl ApprovalBroker {
    pub fn new(store: Store, transcript: Transcript, bus: Bus) -> Self {
        Self {
            store,
            transcript,
            bus,
            waiting: Mutex::new(HashMap::new()),
        }
    }

    /// The waiters, even after a panic while they were held: each entry is whole, and dropping
    /// them all would leave every open card unanswerable.
    fn waiting(&self) -> MutexGuard<'_, HashMap<String, Waiter>> {
        self.waiting.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The decision for `request`: at once if `policy` lets it through, otherwise once the user
    /// answers. The receiver closes unanswered if `engine`'s turn ends first. A request whose
    /// card can't be saved is denied at once, since nothing would be shown to answer it.
    pub fn request(
        &self,
        thread_id: &str,
        engine: u64,
        request: &PermissionRequest,
        policy: ApprovalPolicy,
    ) -> oneshot::Receiver<ApprovalDecision> {
        let (decided, decision) = oneshot::channel();
        let (card, tool) = payload::card(request);
        if !policy.asks_user(request) {
            self.transcript
                .signal(thread_id, payload::event(request, card, true));
            let _ = decided.send(ApprovalDecision::Approved);
            return decision;
        }
        {
            let mut waiting = self.waiting();
            if let Err(error) =
                self.store
                    .insert_pending_approval(&request.id, thread_id, tool, &card)
            {
                log::warn!("saving {thread_id}'s approval {}: {error}", request.id);
                let _ = decided.send(ApprovalDecision::Denied {
                    message: "Cormux couldn't record this request, so it can't be answered.".into(),
                });
                return decision;
            }
            waiting.insert(
                request.id.clone(),
                Waiter {
                    thread_id: thread_id.to_string(),
                    engine,
                    decided,
                },
            );
            if let Err(error) = self
                .transcript
                .append(thread_id, payload::event(request, card, false))
            {
                log::warn!("recording {thread_id}'s approval {}: {error}", request.id);
            }
        }
        self.transcript.signal(
            thread_id,
            HostEvent::CurrentTool {
                id: request.tool_call_id.clone(),
                title: request.tool.clone(),
            },
        );
        self.bus.publish(DomainEvent::ApprovalRequested {
            thread_id: thread_id.to_string(),
            approval_id: request.id.clone(),
            tool: tool.to_string(),
        });
        decision
    }

    /// `engine` stopped waiting (its turn ended, or it exited), so its open requests are denied
    /// and their waiters dropped, and the cards stop asking. Another engine's, such as the one
    /// that replaced it on the same thread, stay open.
    pub fn cancel_for_engine(&self, engine: u64) {
        let mut waiting = self.waiting();
        let ids: Vec<String> = waiting
            .iter()
            .filter(|(_, waiter)| waiter.engine == engine)
            .map(|(id, _)| id.clone())
            .collect();
        let mut threads = std::collections::BTreeSet::new();
        for id in ids {
            let Some(waiter) = waiting.remove(&id) else {
                continue;
            };
            if let Err(error) = self
                .store
                .decide_approval(&id, ApprovalStatus::Denied, None)
            {
                log::warn!("denying approval {id}: {error}");
            }
            threads.insert(waiter.thread_id);
        }
        for thread_id in threads {
            self.bus
                .publish(DomainEvent::ApprovalsCancelled { thread_id });
        }
    }

    /// Records the user's answer and hands it to the waiting engine. An answer to a request
    /// nothing waits on any more is refused, and the card stops asking.
    pub fn resolve(&self, id: &str, answer: Answer) -> Result<()> {
        let mut waiting = self.waiting();
        let row = self
            .store
            .get_approval(id)?
            .ok_or_else(|| Error::Approval(format!("unknown approval {id}")))?;
        if row.status != ApprovalStatus::Pending {
            return Err(Error::Approval("approval already decided".into()));
        }
        let Some(waiter) = waiting.remove(id) else {
            self.store
                .decide_approval(id, ApprovalStatus::Denied, None)?;
            return Err(Error::Approval(
                "the agent is no longer waiting for this answer".into(),
            ));
        };
        let mut card = row.payload;
        if let Some(card) = &mut card {
            card.resolved_at_ms = Some(crate::time::unix_ms());
        }
        let (status, decision) = match answer {
            Answer::Approve => (ApprovalStatus::Approved, ApprovalDecision::Approved),
            Answer::Deny { reason } => {
                let reason = reason.filter(|reason| !reason.trim().is_empty());
                if let Some(card) = &mut card {
                    card.deny_reason = reason.clone();
                }
                let message = reason.unwrap_or_else(|| "Denied by user".into());
                (ApprovalStatus::Denied, ApprovalDecision::Denied { message })
            }
            Answer::Answers(answers) => {
                if let Some(card) = &mut card {
                    card.answers = answers.clone();
                }
                (
                    ApprovalStatus::Approved,
                    ApprovalDecision::Answered { answers },
                )
            }
        };
        // An unreadable card keeps what's stored rather than lose it to a rewrite.
        if let Err(error) = self.store.decide_approval(id, status, card.as_ref()) {
            waiting.insert(id.to_string(), waiter);
            return Err(error);
        }
        if waiter.decided.send(decision).is_err() {
            log::warn!("approval {id} was answered after its engine stopped reading");
        }
        Ok(())
    }

    /// Approvals waiting on the user, which the dock badge shows.
    pub fn pending_count(&self) -> usize {
        self.store.pending_approval_count().unwrap_or_else(|error| {
            log::warn!("counting pending approvals: {error}");
            0
        })
    }
}

#[cfg(test)]
mod tests;
