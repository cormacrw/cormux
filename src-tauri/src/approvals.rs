use crate::error::{Error, Result};

/// Pending permission requests keyed by approval id. Engines block until the UI answers.
#[derive(Debug, Default)]
pub struct ApprovalBroker;

impl ApprovalBroker {
    pub fn new() -> Self {
        Self
    }

    pub async fn resolve(&self, id: &str, approved: bool) -> Result<()> {
        let _ = (id, approved);
        Err(Error::NotImplemented("approvals.resolve".into()))
    }
}
