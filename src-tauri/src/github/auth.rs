use crate::error::{Error, Result};
use crate::shell_env::ShellEnv;
use std::sync::Arc;
use tokio::sync::RwLock;

pub const KEYRING_SERVICE: &str = "cormux";
pub const KEYRING_USER: &str = "github-token";

pub fn read_stored_token() -> Option<String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).ok()?;
    entry.get_password().ok().filter(|token| !token.is_empty())
}

pub fn save_token(token: &str) -> Result<()> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return Err(Error::Github("token must not be empty".into()));
    }
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|error| Error::Github(error.to_string()))?;
    entry
        .set_password(trimmed)
        .map_err(|error| Error::Github(error.to_string()))
}

pub fn clear_token() -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|error| Error::Github(error.to_string()))?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(Error::Github(error.to_string())),
    }
}

pub async fn resolve_token(env: &Arc<RwLock<ShellEnv>>) -> Option<String> {
    if let Some(token) = read_stored_token() {
        return Some(token);
    }
    token_from_gh_cli(env).await
}

async fn token_from_gh_cli(env: &Arc<RwLock<ShellEnv>>) -> Option<String> {
    let env = env.read().await;
    let token = env.run("gh", &["auth", "token"], None).await.ok()?;
    let token = token.trim().to_string();
    if token.is_empty() { None } else { Some(token) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyring_constants_are_stable() {
        assert_eq!(KEYRING_SERVICE, "cormux");
        assert_eq!(KEYRING_USER, "github-token");
    }
}
