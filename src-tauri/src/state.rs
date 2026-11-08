use std::ops::Deref;
use std::sync::Arc;

use crate::app::WorkspaceAppService;
use crate::bus::Bus;
use crate::clickup::ClickupClient;
use crate::engines::{ClaudeEngines, Engines};
use crate::error::Result;
use crate::feedback::{Notifier, UserOutput};
use crate::git::LiveDiffEngine;
use crate::github::Github;
use crate::orchestration::Orchestration;
use crate::paths::DataDir;
use crate::stack::RemoteStackCache;
use crate::thread_runtime::{RuntimeParts, ThreadRuntime};
use crate::transcript::Transcript;
use crate::{
    approvals::ApprovalBroker, git::Git, github::PrSyncScheduler, llm::LlmClient, metrics::Metrics,
    process::ProcessSupervisor, shell_env::SharedEnv, store::Store, workspace_git::WorkspaceGit,
    workspaces::Workspaces,
};

/// The migrations a module writes in Rust, beside the SQL ones `store::migrate` lists. Opening
/// the app's store runs these, so a module's migration runs for users only once it's here.
pub const RUST_MIGRATIONS: &[crate::store::migrate::RustMigration] = &[
    crate::transcript::MIGRATION,
    crate::harness_config::MIGRATION,
    crate::settings::MIGRATION,
    crate::transcript::PERMISSION_CARDS,
];

/// What an owner does to the store when the app opens, for what the last run left mid-way.
pub type OnRestart = fn(&Store) -> Result<()>;

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
    pub shell_env: SharedEnv,
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
    pub github: Github,
    pub pr_sync: PrSyncScheduler,
    pub stacks: RemoteStackCache,
    pub orchestration: Orchestration,
}

impl AppState {
    /// Nothing running survives the app closing: each of `on_restart` puts back what its owner
    /// left mid-way.
    pub fn new(
        data_dir: DataDir,
        output: Arc<dyn UserOutput>,
        on_restart: &[OnRestart],
    ) -> Result<Self> {
        let store = Store::open(&data_dir.db(), RUST_MIGRATIONS)?;
        for restart in on_restart {
            restart(&store)?;
        }
        let shell_env = SharedEnv::loading();
        let engines = Arc::new(ClaudeEngines::new(shell_env.without_waiting()));
        Ok(Self::with_store(
            data_dir, store, output, shell_env, engines,
        ))
    }

    /// Lets go of what the running app keeps for a workspace once it's archived. Each service
    /// that keeps something per workspace forgets it here.
    pub async fn forget_workspace(&self, workspace_id: &str) {
        self.diffs.unwatch(workspace_id);
        self.process.forget_workspace(workspace_id);
        self.workspace_git.forget(workspace_id).await;
        self.stacks.forget(workspace_id);
        self.apps.forget(workspace_id);
    }

    fn with_store(
        data_dir: DataDir,
        store: Store,
        output: Arc<dyn UserOutput>,
        shell_env: SharedEnv,
        engines: Arc<dyn Engines>,
    ) -> Self {
        let git = Git::new(shell_env.clone());
        let diffs = LiveDiffEngine::new(git.clone());
        let github = Github::new(shell_env.clone());
        let process = ProcessSupervisor::new(shell_env.clone());
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
        let apps = WorkspaceAppService::new(
            process.clone(),
            workspaces.clone(),
            store.clone(),
            out.clone(),
        );
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
            SharedEnv::loaded(Default::default()),
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
            SharedEnv::loaded(Default::default()),
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

    /// Bug: the Rust migrations were listed inline here and the version test kept its own list,
    /// so it never saw 030, and a module's migration could pass its own test and never run.
    #[test]
    fn the_app_runs_every_migration_version_once() {
        let versions = crate::store::migrate::versions(RUST_MIGRATIONS);
        let expected: Vec<i64> = (1..=versions.len() as i64).collect();
        assert_eq!(versions, expected);

        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("cormux.db"), RUST_MIGRATIONS).unwrap();
        let applied: Vec<i64> = store
            .with_read(|conn| {
                let mut stmt =
                    conn.prepare("SELECT version FROM schema_migrations ORDER BY version")?;
                let rows = stmt.query_map([], |row| row.get(0))?;
                Ok(rows.collect::<rusqlite::Result<Vec<i64>>>()?)
            })
            .unwrap();
        assert_eq!(applied, expected);
        assert!(applied.contains(&30), "the committed-config migration runs");
    }
}
