use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::sync::RwLock;

use crate::error::{Error, Result};
use crate::shell_env::ShellEnv;

const DEBOUNCE: Duration = Duration::from_secs(60);
/// Upper bound on one `claude -p` call, so a hung CLI can't spin forever.
const CALL_TIMEOUT: Duration = Duration::from_secs(90);

/// One-shot small-model calls for workspace summaries and PR drafting.
#[derive(Clone)]
pub struct LlmClient {
    env: Arc<RwLock<ShellEnv>>,
    last_call: Arc<Mutex<HashMap<String, Instant>>>,
}

impl LlmClient {
    pub fn new(env: Arc<RwLock<ShellEnv>>) -> Self {
        Self {
            env,
            last_call: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn allow(&self, workspace_id: &str) -> bool {
        let mut last = self.last_call.lock().unwrap();
        if let Some(instant) = last.get(workspace_id)
            && instant.elapsed() < DEBOUNCE
        {
            return false;
        }
        last.insert(workspace_id.to_string(), Instant::now());
        true
    }

    pub async fn summarise(&self, workspace_id: &str, prompt: &str) -> Result<Option<String>> {
        if !self.allow(workspace_id) {
            return Ok(None);
        }
        self.complete(prompt).await.map(Some)
    }

    pub async fn complete(&self, prompt: &str) -> Result<String> {
        complete_via_claude(&self.env, prompt).await
    }
}

async fn complete_via_claude(env: &Arc<RwLock<ShellEnv>>, prompt: &str) -> Result<String> {
    // Snapshot the env and release the lock: holding a read guard for the whole
    // CLI run would stall `reload` (write-preferring) and every git call behind it.
    let env = env.read().await.clone();
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
    use crate::shell_env::ShellEnv;

    #[test]
    fn extracts_claude_json_result() {
        let text = extract_text(r#"{"result":"Working on login"}"#);
        assert_eq!(text, "Working on login");
    }

    #[test]
    fn debounce_limits_one_call_per_workspace_per_minute() {
        let env = Arc::new(RwLock::new(ShellEnv::new()));
        let client = LlmClient::new(env);
        assert!(client.allow("ws-1"));
        assert!(!client.allow("ws-1"));
        assert!(client.allow("ws-2"));
    }
}
