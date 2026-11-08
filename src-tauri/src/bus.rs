//! Things that happened in the core, for the features that react to them. Publishers don't know
//! who listens: `turn`, `approvals` and `provisioning` publish; review, orchestration and feedback
//! subscribe from their own `start`.

use std::future::Future;

use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEvent {
    /// A thread's turn is over (or its engine exited) and its status has settled.
    TurnEnded { thread_id: String },
    /// A thread is waiting on the user to answer an approval or a question.
    ApprovalRequested {
        thread_id: String,
        approval_id: String,
        /// The kind of thing it would do, as the approval row names it.
        tool: String,
    },
    /// A workspace's setup failed for good.
    ProvisioningFailed {
        workspace_id: String,
        message: String,
    },
}

#[derive(Clone)]
pub struct Bus(broadcast::Sender<DomainEvent>);

impl Default for Bus {
    fn default() -> Self {
        Self(broadcast::channel(128).0)
    }
}

impl Bus {
    pub fn publish(&self, event: DomainEvent) {
        // Nobody listening is fine: nothing needed to react.
        let _ = self.0.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DomainEvent> {
        self.0.subscribe()
    }

    /// Runs `handle` for every event from now on, one at a time, for the life of the app.
    pub fn listen<F, Fut>(&self, mut handle: F)
    where
        F: FnMut(DomainEvent) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send,
    {
        let mut events = self.subscribe();
        tauri::async_runtime::spawn(async move {
            loop {
                match events.recv().await {
                    Ok(event) => handle(event).await,
                    Err(RecvError::Lagged(missed)) => {
                        log::warn!("a listener missed {missed} core events");
                    }
                    Err(RecvError::Closed) => break,
                }
            }
        });
    }
}
