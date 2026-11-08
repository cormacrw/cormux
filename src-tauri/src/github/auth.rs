use crate::error::{Error, Result};

use super::Gh;

/// The `gh` CLI's login; Cormux keeps no GitHub token of its own.
///
/// # Errors
///
/// Fails when `gh` can't run (missing from PATH, locked keychain) — the
/// underlying message is kept so the user isn't told to log in when that
/// won't help — or when it runs but prints no token.
pub async fn resolve_token(gh: &Gh) -> Result<String> {
    let token = gh.stdout(&["auth", "token"]).await.map_err(|error| {
        Error::Github(format!(
            "could not read the `gh` login (run `gh auth login` if signed out): {error}"
        ))
    })?;
    if token.is_empty() {
        return Err(Error::Github(
            "GitHub is not signed in — run `gh auth login`".into(),
        ));
    }
    Ok(token)
}
