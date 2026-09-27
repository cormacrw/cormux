use crate::error::{Error, Result};

/// GitHub client. Tokens stay in the Keychain and never reach the webview.
#[derive(Debug, Default)]
pub struct GithubClient;

impl GithubClient {
    pub fn new() -> Self {
        Self
    }

    pub async fn sync_open_prs(&self) -> Result<()> {
        Err(Error::NotImplemented("github.sync_open_prs"))
    }
}
