use crate::{
    approvals::ApprovalBroker, engines::EngineRegistry, git::Git, github::GithubClient,
    llm::LlmClient, mcp::HarnessMcp, metrics::Metrics, process::ProcessSupervisor,
    shell_env::ShellEnv, store::Store, workspace::WorkspaceManager,
};

/// Process-wide core state. The webview never holds this; IPC commands borrow it.
pub struct AppState {
    pub shell_env: ShellEnv,
    pub git: Git,
    pub workspace: WorkspaceManager,
    pub engines: EngineRegistry,
    pub approvals: ApprovalBroker,
    pub mcp: HarnessMcp,
    pub process: ProcessSupervisor,
    pub github: GithubClient,
    pub llm: LlmClient,
    pub store: Store,
    pub metrics: Metrics,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            shell_env: ShellEnv::new(),
            git: Git::new(),
            workspace: WorkspaceManager::new(),
            engines: EngineRegistry::new(),
            approvals: ApprovalBroker::new(),
            mcp: HarnessMcp::new(),
            process: ProcessSupervisor::new(),
            github: GithubClient::new(),
            llm: LlmClient::new(),
            store: Store::new(),
            metrics: Metrics::new(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
