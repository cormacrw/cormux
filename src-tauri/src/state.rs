use std::ops::Deref;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::app::WorkspaceAppService;
use crate::bus::Bus;
use crate::clickup::ClickupClient;
use crate::engines::{ClaudeEngines, Engines};
use crate::error::Result;
use crate::feedback::{Notifier, UserOutput};
use crate::git::{FetchScheduler, LiveDiffEngine};
use crate::github::Github;
use crate::orchestration::Orchestration;
use crate::paths::DataDir;
use crate::stack::RemoteStackCache;
use crate::thread_runtime::{RuntimeParts, ThreadRuntime};
use crate::transcript::Transcript;
use crate::{
    approvals::ApprovalBroker, git::Git, github::PrSyncScheduler, llm::LlmClient, metrics::Metrics,
    process::ProcessSupervisor, shell_env::ShellEnv, store::Store, workspace_git::WorkspaceGit,
    workspaces::Workspaces,
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
    pub out: Notifier,
    pub bus: Bus,
    pub shell_env: Arc<RwLock<ShellEnv>>,
    pub git: Git,
    pub workspaces: Workspaces,
    pub workspace_git: WorkspaceGit,
    pub engines: ThreadRuntime,
    pub transcript: Transcript,
    pub approvals: Arc<ApprovalBroker>,
    pub turns: crate::turn::Turns,
    pub setup: crate::provisioning::SetupRuns,
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
    pub orchestration: Orchestration,
}

impl AppState {
    /// Turns that were running when the app last closed are over: no agent survives it.
    pub fn new(data_dir: DataDir, output: Arc<dyn UserOutput>) -> Result<Self> {
        let store = Store::open(
            &data_dir.db(),
            &[
                crate::transcript::MIGRATION,
                crate::harness_config::MIGRATION,
            ],
        )?;
        store.reset_interrupted_turns()?;
        let shell_env = Arc::new(RwLock::new(ShellEnv::new()));
        let engines = Arc::new(ClaudeEngines::new(shell_env.clone()));
        Ok(Self::with_store(
            data_dir, store, output, shell_env, engines,
        ))
    }

    fn with_store(
        data_dir: DataDir,
        store: Store,
        output: Arc<dyn UserOutput>,
        shell_env: Arc<RwLock<ShellEnv>>,
        engines: Arc<dyn Engines>,
    ) -> Self {
        let git = Git::new(shell_env.clone());
        let diffs = LiveDiffEngine::new(git.clone());
        let fetch = FetchScheduler::new(git.clone());
        let github = Github::new(shell_env.clone());
        let process = ProcessSupervisor::new(shell_env.clone());
        let apps = WorkspaceAppService::new();
        let workspaces = Workspaces::new(store.clone());
        let bus = Bus::default();
        let transcript = Transcript::new(store.clone());
        let approvals = Arc::new(ApprovalBroker::new(
            store.clone(),
            transcript.clone(),
            bus.clone(),
        ));
        let out = {
            let approvals = approvals.clone();
            Notifier::new(output, move || approvals.pending_count())
        };
        let engines = ThreadRuntime::new(RuntimeParts {
            engines,
            approvals: approvals.clone(),
            store: store.clone(),
            transcript: transcript.clone(),
            workspaces: workspaces.clone(),
            host_tools: Arc::new(crate::host_tools::session_config),
            brief: {
                let store = store.clone();
                let transcript = transcript.clone();
                Arc::new(move |workspace, thread_id, resuming| {
                    crate::workspace_start::session_brief(
                        &store,
                        &transcript,
                        workspace,
                        thread_id,
                        resuming,
                    )
                })
            },
        });
        Self(Arc::new(Core {
            data_dir,
            out,
            bus,
            shell_env: shell_env.clone(),
            git,
            workspaces,
            workspace_git: WorkspaceGit::default(),
            engines,
            transcript,
            approvals,
            turns: Default::default(),
            setup: Default::default(),
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
            orchestration: Orchestration::default(),
        }))
    }

    #[cfg(test)]
    pub fn recorded(data_dir: DataDir) -> (Self, Arc<crate::feedback::RecordingOutput>) {
        let (state, recording, _) = Self::faked(data_dir);
        (state, recording)
    }

    #[cfg(test)]
    pub fn with_engines(data_dir: DataDir, engines: Arc<dyn Engines>) -> Self {
        Self::with_store(
            data_dir,
            Store::in_memory().expect("in-memory store"),
            Arc::new(crate::feedback::RecordingOutput::default()),
            Arc::new(RwLock::new(ShellEnv::new())),
            engines,
        )
    }

    #[cfg(test)]
    pub fn faked(
        data_dir: DataDir,
    ) -> (
        Self,
        Arc<crate::feedback::RecordingOutput>,
        crate::thread_runtime::fake::FakeEngines,
    ) {
        let recording = Arc::new(crate::feedback::RecordingOutput::default());
        let store = Store::in_memory().expect("in-memory store");
        let fake = crate::thread_runtime::fake::FakeEngines::default();
        let state = Self::with_store(
            data_dir,
            store,
            recording.clone(),
            Arc::new(RwLock::new(ShellEnv::new())),
            Arc::new(fake.clone()),
        );
        (state, recording, fake)
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
