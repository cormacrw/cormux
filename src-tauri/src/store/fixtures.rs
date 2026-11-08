//! Rows for tests, so a new column means one edit here rather than one per test.

use crate::model::{ThreadRole, ThreadStatus, WorkspaceKind, WorkspaceLifecycle};

use super::Store;
use super::types::{RepoRecord, ThreadRow, WorkspaceRow};

/// A repo at `/tmp/<id>` whose default branch is `main`, with no commands.
pub fn repo(id: &str) -> RepoRecord {
    RepoRecord {
        id: id.into(),
        path: format!("/tmp/{id}"),
        name: id.into(),
        default_branch: Some("main".into()),
        setup_commands: String::new(),
        run_command: None,
        single_instance: false,
    }
}

/// A Lead workspace of repo `r1` on branch `<id>`, at `/tmp/<id>`.
pub fn workspace(id: &str, status: WorkspaceLifecycle) -> WorkspaceRow {
    WorkspaceRow::new(
        WorkspaceKind::Lead,
        id,
        "r1",
        id,
        id,
        "main",
        format!("/tmp/{id}"),
    )
    .with_status(status)
}

/// A Lead thread of workspace `workspace_id`.
pub fn thread(id: &str, workspace_id: &str, status: ThreadStatus) -> ThreadRow {
    ThreadRow::new(id, workspace_id, ThreadRole::Lead, "claude").with_status(status)
}

impl Store {
    /// Saves `fixtures::workspace(id, status)`, and repo `r1` for it if it isn't there yet.
    pub fn seed_workspace(&self, id: &str, status: WorkspaceLifecycle) -> WorkspaceRow {
        if self.repo_by_id("r1").unwrap().is_none() {
            self.upsert_repo(&repo("r1")).unwrap();
        }
        let row = workspace(id, status);
        self.insert_workspace(&row).unwrap();
        row
    }

    /// Saves `fixtures::thread(id, workspace_id, status)`.
    pub fn seed_thread(&self, id: &str, workspace_id: &str, status: ThreadStatus) -> ThreadRow {
        let row = thread(id, workspace_id, status);
        self.upsert_thread(&row).unwrap();
        row
    }
}
