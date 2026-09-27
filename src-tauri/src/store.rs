use crate::error::{Error, Result};

/// SQLite persistence in the app data directory.
#[derive(Debug, Default)]
pub struct Store;

impl Store {
    pub fn new() -> Self {
        Self
    }

    pub async fn open(&self) -> Result<()> {
        Err(Error::NotImplemented("store.open".into()))
    }
}
