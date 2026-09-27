use crate::error::{Error, Result};

/// Workspace lifecycle state machine (creating → provisioning → ready → tearing_down).
#[derive(Debug, Default)]
pub struct WorkspaceManager;

impl WorkspaceManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn create(&self) -> Result<()> {
        Err(Error::NotImplemented("workspace.create".into()))
    }
}
