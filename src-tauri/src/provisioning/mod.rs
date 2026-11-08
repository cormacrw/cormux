//! Bringing a workspace into being: its records, then in the background its worktree, setup
//! commands and first turn, as the caller's `ProvisionPlan` says.

mod runner;

pub use runner::{ProvisionPlan, WorkspaceSpec, create, fail, retry, run, skip_setup, start};

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::OwnedMutexGuard;

/// Each workspace's setup run, as teardown sees it. A run carries on only while it is the
/// workspace's newest (Retry and Skip start another) and teardown hasn't cancelled it. Making the
/// worktree is one step under the workspace's lock, which teardown takes to cancel, so teardown
/// never removes a worktree setup is still adding, and setup never adds one teardown missed.
#[derive(Default)]
pub struct SetupRuns {
    current: Mutex<HashMap<String, Arc<tokio::sync::Mutex<u64>>>>,
    started: AtomicU64,
}

/// One setup run of one workspace.
#[derive(Debug, Clone)]
pub struct SetupRun {
    workspace_id: String,
    id: u64,
}

impl SetupRuns {
    fn slot(&self, workspace_id: &str) -> Arc<tokio::sync::Mutex<u64>> {
        self.current
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .entry(workspace_id.to_string())
            .or_default()
            .clone()
    }

    /// The workspace's newest run, waiting for any step of an older one to finish.
    async fn begin(&self, workspace_id: &str) -> SetupRun {
        let id = self.started.fetch_add(1, Ordering::Relaxed) + 1;
        *self.slot(workspace_id).lock_owned().await = id;
        SetupRun {
            workspace_id: workspace_id.to_string(),
            id,
        }
    }

    /// Holds the workspace for one step of `run`, or `None` once the run is over.
    async fn step(&self, run: &SetupRun) -> Option<OwnedMutexGuard<u64>> {
        let held = self.slot(&run.workspace_id).lock_owned().await;
        (*held == run.id).then_some(held)
    }

    /// Ends the workspace's run, once any step it is taking has finished. Nothing more of it
    /// happens while the guard is held, or after.
    pub async fn cancel(&self, workspace_id: &str) -> OwnedMutexGuard<u64> {
        let mut held = self.slot(workspace_id).lock_owned().await;
        *held = 0;
        held
    }
}
