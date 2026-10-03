use crate::shell_env::ShellEnv;
use std::sync::Arc;
use tokio::sync::RwLock;

/// The `gh` CLI's login; Cormux keeps no GitHub token of its own.
pub async fn resolve_token(env: &Arc<RwLock<ShellEnv>>) -> Option<String> {
    let env = env.read().await;
    let token = env.run("gh", &["auth", "token"], None).await.ok()?;
    let token = token.trim().to_string();
    if token.is_empty() { None } else { Some(token) }
}
