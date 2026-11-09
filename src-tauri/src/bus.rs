//! Things that happened in the core, for the features that react to them. Publishers don't know
//! who listens: `turn`, `approvals` and `provisioning` publish; feedback, review, findings and
//! orchestration subscribe from their own `start`, each as its [`Listener`].
//!
//! Every listener hears each event before any listener hears the next, in [`Listener`]'s
//! order whatever order they started in, so two features reacting to one event never race.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEvent {
    /// A thread's turn is over (or its engine exited, or the turn was stopped or its session
    /// replaced) and its status has settled.
    TurnEnded {
        thread_id: String,
        /// What the turn answered and said, as transcript seqs; see [`TurnSpan`].
        span: TurnSpan,
        /// The user stopped it, or replaced its session, before it finished.
        stopped: bool,
    },
    /// Messages held for a paused thread were dropped unsent, because it was stopped or its
    /// session replaced. Each is the seq its message was saved at.
    PromptsDropped {
        thread_id: String,
        message_seqs: Vec<i64>,
    },
    /// A thread is waiting on the user to answer an approval or a question.
    ApprovalRequested {
        thread_id: String,
        approval_id: String,
        /// The kind of thing it would do, as the approval row names it.
        tool: String,
    },
    /// Approvals nobody answered were denied because the engine that asked stopped waiting.
    ApprovalsCancelled { thread_id: String },
    /// A workspace's setup failed for good.
    ProvisioningFailed {
        workspace_id: String,
        message: String,
    },
}

/// Where a turn sits in its thread's transcript: after where it began (`since`: the message that
/// started it, or the turn before's `through` when the engine started it on a message it already
/// had; unknown for a turn the app didn't start), through the last message saved before it ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnSpan {
    pub since: Option<i64>,
    pub through: i64,
}

/// Who listens, in the order each event reaches them. Orchestration starts new turns, so it
/// hears last: review has read a finished turn's reply before orchestration sends that thread its
/// reports, and findings has reopened what a dropped message carried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Listener {
    Feedback,
    Review,
    Findings,
    Orchestration,
}

type Handler = dyn Fn(DomainEvent) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync;

/// Each listener's handler, in `Listener` order.
type Listeners = Arc<Mutex<Vec<(Listener, Arc<Handler>)>>>;

#[derive(Clone)]
pub struct Bus {
    sender: broadcast::Sender<DomainEvent>,
    listeners: Listeners,
    /// Events sent to the dispatcher, and how many its listeners have finished with.
    published: Arc<std::sync::atomic::AtomicU64>,
    handled: Arc<tokio::sync::watch::Sender<u64>>,
}

impl Default for Bus {
    fn default() -> Self {
        Self {
            sender: broadcast::channel(128).0,
            listeners: Arc::default(),
            published: Arc::default(),
            handled: Arc::new(tokio::sync::watch::Sender::new(0)),
        }
    }
}

impl Bus {
    pub fn publish(&self, event: DomainEvent) {
        let listening = !self
            .listeners
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .is_empty();
        // Nobody listening is fine: nothing needed to react.
        if self.sender.send(event).is_ok() && listening {
            self.published
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    /// Waits until every listener has finished with every event published so far.
    #[cfg(test)]
    pub async fn settled(&self) {
        let published = self.published.load(std::sync::atomic::Ordering::SeqCst);
        let mut handled = self.handled.subscribe();
        let _ = handled.wait_for(|handled| *handled >= published).await;
    }

    /// The listeners, in the order each event reaches them.
    #[cfg(test)]
    pub fn listeners(&self) -> Vec<Listener> {
        self.listeners
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .iter()
            .map(|(listener, _)| *listener)
            .collect()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DomainEvent> {
        self.sender.subscribe()
    }

    /// Runs `handle` for every event from now on, in `listener`'s place among the others, for the
    /// life of the app.
    pub fn listen<F, Fut>(&self, listener: Listener, handle: F)
    where
        F: Fn(DomainEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handler: Arc<Handler> = Arc::new(move |event| Box::pin(handle(event)));
        let first = {
            let mut listeners = self
                .listeners
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let at = listeners.partition_point(|(other, _)| *other <= listener);
            listeners.insert(at, (listener, handler));
            listeners.len() == 1
        };
        if first {
            self.dispatch();
        }
    }

    fn dispatch(&self) {
        let mut events = self.subscribe();
        let listeners = self.listeners.clone();
        let handled = self.handled.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                match events.recv().await {
                    Ok(event) => {
                        let handlers: Vec<Arc<Handler>> = listeners
                            .lock()
                            .unwrap_or_else(|error| error.into_inner())
                            .iter()
                            .map(|(_, handle)| handle.clone())
                            .collect();
                        for handle in handlers {
                            handle(event.clone()).await;
                        }
                        handled.send_modify(|handled| *handled += 1);
                    }
                    Err(RecvError::Lagged(missed)) => {
                        log::warn!("the core's listeners missed {missed} events");
                        handled.send_modify(|handled| *handled += missed);
                    }
                    Err(RecvError::Closed) => break,
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bug: each listener ran in its own task, so orchestration could send a Lead its reports
    /// before review had read the reply the Lead's turn ended with. The order is the
    /// `Listener`s', not the order they started listening in.
    #[tokio::test]
    async fn each_event_reaches_every_listener_in_turn_before_the_next() {
        let bus = Bus::default();
        let heard = Arc::new(Mutex::new(Vec::new()));
        for (name, listener) in [("second", Listener::Review), ("first", Listener::Feedback)] {
            let heard = heard.clone();
            bus.listen(listener, move |event| {
                let heard = heard.clone();
                async move {
                    if name == "first" {
                        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                    }
                    if let DomainEvent::ApprovalsCancelled { thread_id } = event {
                        heard.lock().unwrap().push(format!("{name} {thread_id}"));
                    }
                }
            });
        }

        for thread_id in ["a", "b"] {
            bus.publish(DomainEvent::ApprovalsCancelled {
                thread_id: thread_id.into(),
            });
        }
        bus.settled().await;

        assert_eq!(
            *heard.lock().unwrap(),
            ["first a", "second a", "first b", "second b"]
        );
    }
}
