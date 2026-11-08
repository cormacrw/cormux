use std::ops::Deref;
use std::sync::Arc;

use tokio::sync::{RwLock, broadcast};

use crate::app::WorkspaceAppService;
use crate::clickup::ClickupClient;
use crate::feedback::{Notifier, UserOutput};
use crate::git::{FetchScheduler, LiveDiffEngine};
use crate::orchestration::ReportQueue;
use crate::paths::DataDir;
use crate::{
    approvals::ApprovalBroker, engines::EngineRegistry, git::Git, github::PrSyncScheduler,
    llm::LlmClient, mcp::CormuxMcp, metrics::Metrics, process::ProcessSupervisor,
    shell_env::ShellEnv, store::Store, workspace::WorkspaceManager,
};

/// Process-wide core state. The webview never holds this; IPC commands borrow it. A handle to
/// one shared [`Core`], so a workflow can clone it into the background tasks it spawns.
#[derive(Clone)]
pub struct AppState(Arc<Core>);

impl Deref for AppState {
    type Target = Core;

    fn deref(&self) -> &Core {
        &self.0
    }
}

pub struct Core {
    pub data_dir: DataDir,
    /// What workflows tell the user through.
    pub out: Notifier,
    pub shell_env: Arc<RwLock<ShellEnv>>,
    pub git: Git,
    pub workspace: WorkspaceManager,
    pub engines: EngineRegistry,
    pub approvals: Arc<ApprovalBroker>,
    pub mcp: CormuxMcp,
    pub process: ProcessSupervisor,
    pub apps: WorkspaceAppService,
    pub clickup: ClickupClient,
    pub llm: LlmClient,
    pub store: Store,
    pub metrics: Metrics,
    pub diffs: LiveDiffEngine,
    pub fetch: FetchScheduler,
    pub pr_sync: PrSyncScheduler,
    pub approval_notify: broadcast::Sender<()>,
    pub turn_end_notify: broadcast::Sender<String>,
    pub reports: ReportQueue,
}

impl AppState {
    pub fn new(data_dir: DataDir, output: Arc<dyn UserOutput>) -> Self {
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
        let out = {
            let (store, approvals) = (store.clone(), approvals.clone());
            Notifier::new(output, move || {
                crate::approvals::pending_count(&store, &approvals)
            })
        };
        let (approval_notify, _) = broadcast::channel(32);
        let (turn_end_notify, _) = broadcast::channel(64);
        let mcp = CormuxMcp::new(store.clone());
        let engines = EngineRegistry::new(
            shell_env.clone(),
            approvals.clone(),
            store.clone(),
            mcp.clone(),
            approval_notify.clone(),
            turn_end_notify.clone(),
        );
        Self(Arc::new(Core {
            data_dir,
            out,
            shell_env: shell_env.clone(),
            git,
            workspace,
            engines,
            approvals,
            mcp,
            process,
            apps,
            clickup: ClickupClient::new(),
            llm: LlmClient::new(shell_env),
            store,
            metrics: Metrics::new(),
            diffs,
            fetch,
            pr_sync,
            approval_notify,
            turn_end_notify,
            reports: ReportQueue::default(),
        }))
    }

    /// A state whose output is recorded, for tests to assert on.
    #[cfg(test)]
    pub fn recorded(data_dir: DataDir) -> (Self, Arc<crate::feedback::RecordingOutput>) {
        let recording = Arc::new(crate::feedback::RecordingOutput::default());
        (Self::new(data_dir, recording.clone()), recording)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_without_async_runtime() {
        let _state = AppState::recorded(DataDir::new(std::env::temp_dir()));
    }
}
