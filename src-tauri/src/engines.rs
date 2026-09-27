use crate::error::{Error, Result};

/// Adapters that spawn engine CLIs and translate ACP / stream-JSON into one event model.
#[derive(Debug, Default)]
pub struct EngineRegistry;

impl EngineRegistry {
    pub fn new() -> Self {
        Self
    }

    pub async fn detect(&self) -> Result<()> {
        Err(Error::NotImplemented("engines.detect".into()))
    }
}
