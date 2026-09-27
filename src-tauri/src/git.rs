use crate::error::{Error, Result};

/// Shells out to the system `git` so credential helpers, hooks and LFS apply.
#[derive(Debug, Default)]
pub struct Git;

impl Git {
    pub fn new() -> Self {
        Self
    }

    pub async fn validate_repo(&self, path: &str) -> Result<String> {
        let _ = path;
        Err(Error::NotImplemented("git.validate_repo".into()))
    }
}
