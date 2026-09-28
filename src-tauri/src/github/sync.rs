use std::sync::Arc;
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

use super::auth;
use super::client::OpenPrsClient;
use super::r#match::load_repo_origins;

const SYNC_INTERVAL: Duration = Duration::from_secs(120);
const SYNCED_AT_KEY: &str = "githubPrSyncedAt";

#[derive(Clone)]
pub struct PrSyncScheduler {
    git: Git,
    client: OpenPrsClient,
    shell_env: Arc<RwLock<ShellEnv>>,
}

impl PrSyncScheduler {
    pub fn new(git: Git, shell_env: Arc<RwLock<ShellEnv>>) -> Self {
        Self {
            git,
            client: OpenPrsClient::new(),
            shell_env,
        }
    }

    pub async fn tick(&self, state: &AppState) -> Result<bool> {
        let token = match auth::resolve_token(&self.shell_env).await {
            Some(token) => token,
            None => {
                state.store.replace_pull_requests(&[])?;
                state.store.set_setting(SYNCED_AT_KEY, "")?;
                return Ok(true);
            }
        };

        let repos: Vec<(String, String)> = state
            .store
            .snapshot()?
            .repos
            .iter()
            .map(|repo| (repo.id.clone(), repo.path.clone()))
            .collect();
        let origins = load_repo_origins(&self.git, &repos).await?;
        let (items, rate) = self.client.fetch_open_prs(&token, &origins).await?;

        if rate.remaining == Some(0) {
            log::warn!("GitHub rate limit exhausted after sync");
        }

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
        Ok(true)
    }

    pub fn spawn_loop(self, app: AppHandle) {
        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(SYNC_INTERVAL);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
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
        let _changed = self.tick(&state).await?;
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
