use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;
use tokio::sync::RwLock;

use crate::error::{Error, Result};
use crate::shell_env::ShellEnv;

const DEBOUNCE: Duration = Duration::from_secs(60);
const KEYRING_SERVICE: &str = "cormux";
const KEYRING_USER: &str = "anthropic-api-key";

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LlmResult {
    pub text: String,
    pub source: String,
}

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

    pub async fn summarise(&self, workspace_id: &str, prompt: &str) -> Result<Option<LlmResult>> {
        if !self.allow(workspace_id) {
            return Ok(None);
        }
        self.complete(prompt, 256).await.map(Some)
    }

    pub async fn complete(&self, prompt: &str, max_tokens: u32) -> Result<LlmResult> {
        if let Some(key) = read_api_key() {
            return complete_via_api(&key, prompt, max_tokens).await;
        }
        complete_via_claude(&self.env, prompt).await
    }
}

fn read_api_key() -> Option<String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).ok()?;
    entry.get_password().ok().filter(|key| !key.is_empty())
}

async fn complete_via_claude(env: &Arc<RwLock<ShellEnv>>, prompt: &str) -> Result<LlmResult> {
    let env = env.read().await;
    let stdout = env
        .run_with_stdin(
            "claude",
            &["-p", "--model", "haiku", "--output-format", "json"],
            None,
            Some(prompt),
        )
        .await
        .map_err(|error| Error::Llm(error.to_string()))?;
    Ok(LlmResult {
        text: extract_text(&stdout),
        source: "claude-cli".into(),
    })
}

async fn complete_via_api(api_key: &str, prompt: &str, max_tokens: u32) -> Result<LlmResult> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": "claude-haiku-4-5",
        "max_tokens": max_tokens,
        "messages": [{"role": "user", "content": prompt}]
    });
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|error| Error::Llm(error.to_string()))?;
    if !response.status().is_success() {
        return Err(Error::Llm(format!("anthropic http {}", response.status())));
    }
    let value: Value = response
        .json()
        .await
        .map_err(|error| Error::Llm(error.to_string()))?;
    let text = value
        .pointer("/content/0/text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    Ok(LlmResult {
        text,
        source: "anthropic-api".into(),
    })
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
