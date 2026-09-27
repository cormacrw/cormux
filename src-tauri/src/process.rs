use crate::error::{Error, Result};

/// Setup and run commands in a PTY, each in its own process group.
#[derive(Debug, Default)]
pub struct ProcessSupervisor;

impl ProcessSupervisor {
    pub fn new() -> Self {
        Self
    }

    pub async fn stop_all(&self) -> Result<()> {
        Err(Error::NotImplemented("process.stop_all"))
    }
}
