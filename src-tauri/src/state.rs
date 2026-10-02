use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::{RwLock, broadcast};

use crate::app::WorkspaceAppService;
use crate::clickup::ClickupClient;
use crate::git::{FetchScheduler, LiveDiffEngine};
use crate::ipc::subscriptions::Subscriptions;
use crate::{
    approvals::ApprovalBroker, engines::EngineRegistry, git::Git, github::GithubClient,
    github::PrSyncScheduler, llm::LlmClient, mcp::CormuxMcp, metrics::Metrics,
    process::ProcessSupervisor, shell_env::ShellEnv, store::Store, workspace::WorkspaceManager,
};

/// Process-wide core state. The webview never holds this; IPC commands borrow it.
pub struct AppState {
    pub event_version: AtomicU64,
    pub shell_env: Arc<RwLock<ShellEnv>>,
    pub git: Git,
    pub workspace: WorkspaceManager,
    pub engines: EngineRegistry,
    pub approvals: Arc<ApprovalBroker>,
    pub mcp: CormuxMcp,
    pub process: ProcessSupervisor,
    pub apps: WorkspaceAppService,
    pub github: GithubClient,
    pub clickup: ClickupClient,
    pub llm: LlmClient,
    pub store: Store,
    pub metrics: Metrics,
    pub diffs: LiveDiffEngine,
    pub fetch: FetchScheduler,
    pub pr_sync: PrSyncScheduler,
    pub approval_notify: broadcast::Sender<()>,
    pub turn_end_notify: broadcast::Sender<String>,
    pub subscriptions: Subscriptions,
}

impl AppState {
    pub fn new() -> Self {
        let shell_env = Arc::new(RwLock::new(ShellEnv::new()));
        let git = Git::new(shell_env.clone());
        let diffs = LiveDiffEngine::new(git.clone());
        let fetch = FetchScheduler::new(git.clone());
        let pr_sync = PrSyncScheduler::new(git.clone(), shell_env.clone());
        let process = ProcessSupervisor::new(shell_env.clone());
        let apps = WorkspaceAppService::new();
        let workspace = WorkspaceManager::new(git.clone(), diffs.clone(), fetch.clone());
        let store = Store::new();
        let approvals = Arc::new(ApprovalBroker::new());
        let (approval_notify, _) = broadcast::channel(32);
        let (turn_end_notify, _) = broadcast::channel(64);
        let engines = EngineRegistry::new(
            shell_env.clone(),
            approvals.clone(),
            store.clone(),
            approval_notify.clone(),
            turn_end_notify.clone(),
        );
        let mcp = CormuxMcp::new(
            store.clone(),
            process.clone(),
            approvals.clone(),
            apps.clone(),
        );
        Self {
            event_version: AtomicU64::new(0),
            shell_env: shell_env.clone(),
            git,
            workspace,
            engines,
            approvals,
            mcp,
            process,
            apps,
            github: GithubClient::new(),
            clickup: ClickupClient::new(),
            llm: LlmClient::new(shell_env),
            store,
            metrics: Metrics::new(),
            diffs,
            fetch,
            pr_sync,
            approval_notify,
            turn_end_notify,
            subscriptions: Subscriptions::new(),
        }
    }

    pub fn snapshot_version(&self) -> u64 {
        self.event_version.load(Ordering::SeqCst)
    }

    pub fn bump_event_version(&self) -> u64 {
        self.event_version.fetch_add(1, Ordering::SeqCst) + 1
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn constructs_without_async_runtime() {
        let _state = super::AppState::new();
    }
}
