use std::collections::HashMap;
use std::sync::Mutex;

use tokio::sync::oneshot;

use crate::error::{Error, Result};

/// Pending permission requests keyed by approval id. Engine adapters block on the
/// receiver until the UI (or auto-approve) resolves them.
#[derive(Debug, Default)]
pub struct ApprovalBroker {
    pending: Mutex<HashMap<String, oneshot::Sender<ApprovalDecision>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalDecision {
    Approved,
    Denied { message: String },
}

impl ApprovalBroker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, id: impl Into<String>) -> oneshot::Receiver<ApprovalDecision> {
        let (tx, rx) = oneshot::channel();
        let mut pending = self.pending.lock().expect("approval broker mutex");
        pending.insert(id.into(), tx);
        rx
    }

    pub async fn wait(&self, id: impl Into<String>) -> Result<ApprovalDecision> {
        let rx = self.register(id);
        rx.await
            .map_err(|_| Error::Approval("approval dropped".into()))
    }

    pub async fn resolve(&self, id: &str, approved: bool) -> Result<()> {
        let decision = if approved {
            ApprovalDecision::Approved
        } else {
            ApprovalDecision::Denied {
                message: "Denied by user".into(),
            }
        };
        self.complete(id, decision)
    }

    pub fn complete(&self, id: &str, decision: ApprovalDecision) -> Result<()> {
        let mut pending = self
            .pending
            .lock()
            .map_err(|error| Error::Approval(error.to_string()))?;
        let Some(tx) = pending.remove(id) else {
            return Err(Error::Approval(format!("no pending approval {id}")));
        };
        tx.send(decision)
            .map_err(|_| Error::Approval("engine is no longer waiting".into()))?;
        Ok(())
    }

    pub fn pending_count(&self) -> usize {
        self.pending.lock().map(|map| map.len()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn resolve_unblocks_waiter() {
        let broker = ApprovalBroker::new();
        let rx = broker.register("a1");
        broker.resolve("a1", true).await.unwrap();
        assert_eq!(rx.await.unwrap(), ApprovalDecision::Approved);
        assert_eq!(broker.pending_count(), 0);
    }
}
