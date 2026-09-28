pub mod auth;
mod client;
mod r#match;
mod sync;
pub mod types;

pub use auth::{clear_token, save_token};
pub use sync::PrSyncScheduler;

use crate::error::Result;

/// GitHub integration entry point. Tokens stay in the Keychain and never reach the webview.
#[derive(Debug, Default, Clone)]
pub struct GithubClient;

impl GithubClient {
    pub fn new() -> Self {
        Self
    }

    pub async fn sync_open_prs(&self) -> Result<()> {
        Err(crate::error::Error::NotImplemented(
            "use PrSyncScheduler::tick".into(),
        ))
    }
}
