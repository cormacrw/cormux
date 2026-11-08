//! One-shot small-model calls through the `claude` CLI. Callers own their prompts and how
//! often they ask.

use std::time::Duration;

use serde_json::Value;

use crate::error::{Error, Result};
use crate::shell_env::SharedEnv;

/// Upper bound on one `claude -p` call, so a hung CLI can't spin forever.
const CALL_TIMEOUT: Duration = Duration::from_secs(90);

#[derive(Clone)]
pub struct LlmClient {
    env: SharedEnv,
}

impl LlmClient {
    pub fn new(env: SharedEnv) -> Self {
        Self { env }
    }

    pub async fn complete(&self, prompt: &str) -> Result<String> {
        let env = self.env.current().await;
        let call = env.run_with_stdin(
            "claude",
            &["-p", "--model", "haiku", "--output-format", "json"],
            None,
            Some(prompt),
        );
        let stdout = tokio::time::timeout(CALL_TIMEOUT, call)
            .await
            .map_err(|_| Error::Llm(format!("claude -p timed out after {CALL_TIMEOUT:?}")))?
            .map_err(|error| Error::Llm(error.to_string()))?;
        Ok(extract_text(&stdout))
    }
}

pub fn extract_text(stdout: &str) -> String {
    if let Ok(value) = serde_json::from_str::<Value>(stdout) {
        if let Some(text) = value.get("result").and_then(Value::as_str) {
            return text.to_string();
        }
        if let Some(text) = value.pointer("/content/0/text").and_then(Value::as_str) {
            return text.to_string();
        }
    }
    stdout.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_claude_json_result() {
        let text = extract_text(r#"{"result":"Working on login"}"#);
        assert_eq!(text, "Working on login");
    }
}
