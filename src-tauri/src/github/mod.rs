pub mod auth;
mod client;
pub mod create;
pub mod gh;
mod r#match;
mod rest;
pub mod review;
mod store;
mod sync;
pub mod types;

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::shell_env::ShellEnv;

pub use gh::Gh;
pub use r#match::RepoSlug;
pub use rest::RestGithubClient;
pub use store::PrRow;
pub(crate) use store::all as pull_requests_in;
pub use sync::{PrSyncScheduler, start};

/// GitHub's two transports, made once: the `gh` CLI (reads, login) and REST (writes).
#[derive(Clone)]
pub struct Github {
    pub gh: Gh,
    pub rest: RestGithubClient,
}

impl Github {
    pub fn new(env: Arc<RwLock<ShellEnv>>) -> Self {
        Self {
            gh: Gh::new(env),
            rest: RestGithubClient::new(),
        }
    }
}
