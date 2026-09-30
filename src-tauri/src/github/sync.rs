use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tokio::sync::RwLock;

use crate::error::Result;
use crate::git::Git;
use crate::ipc::events::StateChanged;
use crate::ipc::types::StateChangeKind;
use crate::shell_env::ShellEnv;
use crate::state::AppState;
use crate::store::types::PrRow;
use tauri_specta::Event;

use super::client::{self, FetchError};
use super::r#match::load_repo_origins;

const SYNC_INTERVAL: Duration = Duration::from_secs(120);
const SYNCED_AT_KEY: &str = "githubPrSyncedAt";

const AUTH_UNKNOWN: u8 = 0;
const AUTH_SIGNED_IN: u8 = 1;
const AUTH_SIGNED_OUT: u8 = 2;

#[derive(Clone)]
pub struct PrSyncScheduler {
    git: Git,
    shell_env: Arc<RwLock<ShellEnv>>,
    /// Outcome of the last sync that reached GitHub; read by snapshots instead of shelling out.
    auth: Arc<AtomicU8>,
}

impl PrSyncScheduler {
    pub fn new(git: Git, shell_env: Arc<RwLock<ShellEnv>>) -> Self {
        Self {
            git,
            shell_env,
            auth: Arc::new(AtomicU8::new(AUTH_UNKNOWN)),
        }
    }

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
        // Until the login-shell env loads, `gh` and `git` aren't on PATH for a GUI launch.
        if self.shell_env.read().await.vars().is_empty() {
            return Ok(false);
        }

        let repos: Vec<(String, String)> = state
            .store
            .snapshot()?
            .repos
            .iter()
            .map(|repo| (repo.id.clone(), repo.path.clone()))
            .collect();
        let origins = load_repo_origins(&self.git, &repos).await?;
        let items = match client::fetch_open_prs(&self.shell_env, &origins).await {
            Ok(items) => items,
            Err(FetchError::SignedOut(message)) => {
                log::info!("GitHub CLI is signed out; clearing PRs: {message}");
                self.auth.store(AUTH_SIGNED_OUT, Ordering::SeqCst);
                state.store.replace_pull_requests(&[])?;
                state.store.set_setting(SYNCED_AT_KEY, "")?;
                return Ok(true);
            }
            Err(FetchError::Other(error)) => return Err(error),
        };

        let mut rows = Vec::with_capacity(items.len());
        for pr in &items {
            let payload = serde_json::to_string(pr)
                .map_err(|error| crate::error::Error::Github(error.to_string()))?;
            rows.push(PrRow {
                id: pr.cache_id(),
                repo_id: pr.repo_id.clone(),
                number: pr.num,
                title: pr.title.clone(),
                payload,
            });
        }

        state.store.replace_pull_requests(&rows)?;
        let synced_at = chrono_like_now();
        state.store.set_setting(SYNCED_AT_KEY, &synced_at)?;
        self.auth.store(AUTH_SIGNED_IN, Ordering::SeqCst);
        Ok(true)
    }

    pub fn spawn_loop(self, app: AppHandle) {
        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(SYNC_INTERVAL);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            // The first tick fires immediately; the caller runs the initial sync itself.
            interval.tick().await;
            loop {
                interval.tick().await;
                if let Err(error) = self.sync_app(&app).await {
                    log::warn!("background PR sync failed: {error}");
                }
            }
        });
    }

    pub async fn sync_app(&self, app: &AppHandle) -> Result<()> {
        let state = app.state::<AppState>();
        if !self.tick(&state).await? {
            return Ok(());
        }
        let version = state.bump_event_version();
        let _ = StateChanged {
            version,
            kind: StateChangeKind::PrSync,
        }
        .emit(app);
        Ok(())
    }
}

fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}
