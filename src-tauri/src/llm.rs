use crate::error::{Error, Result};

/// One-shot small-model calls for workspace summaries and PR drafting.
#[derive(Debug, Default)]
pub struct LlmClient;

impl LlmClient {
    pub fn new() -> Self {
        Self
    }

    pub async fn summarise(&self, prompt: &str) -> Result<String> {
        let _ = prompt;
        Err(Error::NotImplemented("llm.summarise".into()))
    }
}
