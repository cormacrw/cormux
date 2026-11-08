use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

use crate::error::Result;
use crate::state::AppState;

use super::client::{self, FetchError};
use super::r#match::load_repo_origins;

const SYNC_INTERVAL: Duration = Duration::from_secs(120);
const SYNCED_AT_KEY: &str = "githubPrSyncedAt";

const AUTH_SIGNED_IN: u8 = 1;
const AUTH_SIGNED_OUT: u8 = 2;

#[derive(Clone, Default)]
pub struct PrSyncScheduler {
    /// Outcome of the last sync that reached GitHub, 0 until one has; read by snapshots instead
    /// of shelling out.
    auth: Arc<AtomicU8>,
}

impl PrSyncScheduler {
    /// Before the first sync of this launch, trust the last launch: a stored sync time
    /// means we were signed in, and it is cleared on sign-out.
    pub fn auth_configured(&self, state: &AppState) -> bool {
        match self.auth.load(Ordering::SeqCst) {
            AUTH_SIGNED_IN => true,
            AUTH_SIGNED_OUT => false,
            _ => state
                .store
                .get_setting(SYNCED_AT_KEY)
                .ok()
                .flatten()
                .is_some_and(|value| !value.is_empty()),
        }
    }

    /// Returns whether the cache changed. Failures keep the last good list; only a
    /// definite signed-out answer from `gh` clears it.
    pub async fn tick(&self, state: &AppState) -> Result<bool> {
        let repos: Vec<(String, String)> = state
            .store
            .repos()?
            .into_iter()
            .map(|repo| (repo.id, repo.path))
            .collect();
        let origins = load_repo_origins(&state.git, &repos).await?;
        let items = match client::fetch_open_prs(&state.github.gh, &origins).await {
            // Only PRs in repos registered in Settings; the search itself spans every repo.
            Ok(items) => items
                .into_iter()
                .filter(|pr| pr.repo_id.is_some())
                .collect::<Vec<_>>(),
            Err(FetchError::SignedOut(message)) => {
                log::info!("GitHub CLI is signed out; clearing PRs: {message}");
                self.auth.store(AUTH_SIGNED_OUT, Ordering::SeqCst);
                let changed = state.store.replace_pull_requests(&[])?;
                state.store.set_setting(SYNCED_AT_KEY, "")?;
                return Ok(changed);
            }
            Err(FetchError::Other(error)) => return Err(error),
        };

        let changed = state.store.replace_pull_requests(&items)?;
        let synced_at = crate::time::unix_secs().to_string();
        state.store.set_setting(SYNCED_AT_KEY, &synced_at)?;
        self.auth.store(AUTH_SIGNED_IN, Ordering::SeqCst);
        Ok(changed)
    }

    pub async fn sync_app(&self, state: &AppState) -> Result<()> {
        if !self.tick(state).await? {
            return Ok(());
        }
        state.out.changed();
        Ok(())
    }
}

pub fn start(state: &AppState) {
    let state = state.clone();
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(SYNC_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            if let Err(error) = state.pr_sync.sync_app(&state).await {
                log::warn!("PR sync failed: {error}");
            }
        }
    });
}
