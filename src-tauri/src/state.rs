use std::sync::atomic::{AtomicU64, Ordering};

use crate::{
    approvals::ApprovalBroker, engines::EngineRegistry, git::Git, github::GithubClient,
    llm::LlmClient, mcp::CormuxMcp, metrics::Metrics, process::ProcessSupervisor,
    shell_env::ShellEnv, store::Store, workspace::WorkspaceManager,
};

/// Process-wide core state. The webview never holds this; IPC commands borrow it.
pub struct AppState {
    pub event_version: AtomicU64,
    pub shell_env: ShellEnv,
    pub git: Git,
    pub workspace: WorkspaceManager,
    pub engines: EngineRegistry,
    pub approvals: ApprovalBroker,
    pub mcp: CormuxMcp,
    pub process: ProcessSupervisor,
    pub github: GithubClient,
    pub llm: LlmClient,
    pub store: Store,
    pub metrics: Metrics,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            event_version: AtomicU64::new(0),
            shell_env: ShellEnv::new(),
            git: Git::new(),
            workspace: WorkspaceManager::new(),
            engines: EngineRegistry::new(),
            approvals: ApprovalBroker::new(),
            mcp: CormuxMcp::new(),
            process: ProcessSupervisor::new(),
            github: GithubClient::new(),
            llm: LlmClient::new(),
            store: Store::new(),
            metrics: Metrics::new(),
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
