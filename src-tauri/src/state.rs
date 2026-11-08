use std::ops::Deref;
use std::sync::Arc;

use tokio::sync::{RwLock, broadcast};

use crate::app::WorkspaceAppService;
use crate::clickup::ClickupClient;
use crate::error::Result;
use crate::feedback::{Notifier, UserOutput};
use crate::git::{FetchScheduler, LiveDiffEngine};
use crate::github::Github;
use crate::orchestration::ReportQueue;
use crate::paths::DataDir;
use crate::stack::RemoteStackCache;
use crate::summaries::Summaries;
use crate::{
    approvals::ApprovalBroker, engines::EngineRegistry, git::Git, github::PrSyncScheduler,
    llm::LlmClient, mcp::CormuxMcp, metrics::Metrics, process::ProcessSupervisor,
    shell_env::ShellEnv, store::Store, workspace_git::WorkspaceGit, workspaces::Workspaces,
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
    pub workspaces: Workspaces,
    pub workspace_git: WorkspaceGit,
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
    pub github: Github,
    pub pr_sync: PrSyncScheduler,
    pub stacks: RemoteStackCache,
    pub summaries: Summaries,
    pub approval_notify: broadcast::Sender<()>,
    pub turn_end_notify: broadcast::Sender<String>,
    pub setup_failed: broadcast::Sender<crate::provisioning::SetupFailed>,
    pub reports: ReportQueue,
}

impl AppState {
    /// The app's state over the database in `data_dir`. Turns that were running when the app
    /// last closed are over: no agent survives it.
    pub fn new(data_dir: DataDir, output: Arc<dyn UserOutput>) -> Result<Self> {
        let store = Store::open(&data_dir.db())?;
        store.reset_interrupted_turns()?;
        Ok(Self::with_store(data_dir, store, output))
    }

    fn with_store(data_dir: DataDir, store: Store, output: Arc<dyn UserOutput>) -> Self {
        let shell_env = Arc::new(RwLock::new(ShellEnv::new()));
        let git = Git::new(shell_env.clone());
        let diffs = LiveDiffEngine::new(git.clone());
        let fetch = FetchScheduler::new(git.clone());
        let github = Github::new(shell_env.clone());
        let process = ProcessSupervisor::new(shell_env.clone());
        let apps = WorkspaceAppService::new();
        let workspaces = Workspaces::new(store.clone());
        let approvals = Arc::new(ApprovalBroker::new());
        let out = {
            let (store, approvals) = (store.clone(), approvals.clone());
            Notifier::new(output, move || {
                crate::approvals::pending_count(&store, &approvals)
            })
        };
        let (approval_notify, _) = broadcast::channel(32);
        let (turn_end_notify, _) = broadcast::channel(64);
        let (setup_failed, _) = broadcast::channel(16);
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
            workspaces,
            workspace_git: WorkspaceGit::default(),
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
            github,
            pr_sync: PrSyncScheduler::default(),
            stacks: RemoteStackCache::default(),
            summaries: Summaries::default(),
            approval_notify,
            turn_end_notify,
            setup_failed,
            reports: ReportQueue::default(),
        }))
    }

    /// A state whose output is recorded, for tests to assert on.
    #[cfg(test)]
    pub fn recorded(data_dir: DataDir) -> (Self, Arc<crate::feedback::RecordingOutput>) {
        let recording = Arc::new(crate::feedback::RecordingOutput::default());
        let store = Store::in_memory().expect("in-memory store");
        (
            Self::with_store(data_dir, store, recording.clone()),
            recording,
        )
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
