use crate::error::{Error, Result};

/// Sampled resident memory for Harness, the webview, and child process trees.
#[derive(Debug, Default)]
pub struct Metrics;

impl Metrics {
    pub fn new() -> Self {
        Self
    }

    pub async fn sample(&self) -> Result<u64> {
        Err(Error::NotImplemented("metrics.sample".into()))
    }
}
