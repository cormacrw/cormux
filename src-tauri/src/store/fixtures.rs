//! Rows for tests, so a new column means one edit here rather than one per test.

use crate::model::{ThreadRole, ThreadStatus, WorkspaceKind, WorkspaceLifecycle};

use super::Store;
use super::types::{RepoRecord, ThreadRow, WorkspaceRow};

pub fn repo(id: &str) -> RepoRecord {
    RepoRecord {
        id: id.into(),
        path: format!("/tmp/{id}"),
        name: id.into(),
        default_branch: "main".into(),
        setup_commands: String::new(),
        run_command: None,
        single_instance: false,
    }
}

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

pub fn thread(id: &str, workspace_id: &str, status: ThreadStatus) -> ThreadRow {
    ThreadRow::new(id, workspace_id, ThreadRole::Lead, "claude").with_status(status)
}

impl Store {
    pub fn seed_workspace(&self, id: &str, status: WorkspaceLifecycle) -> WorkspaceRow {
        if self.repo_by_id("r1").unwrap().is_none() {
            self.upsert_repo(&repo("r1")).unwrap();
        }
        let row = workspace(id, status);
        self.insert_workspace(&row).unwrap();
        row
    }

    pub fn seed_thread(&self, id: &str, workspace_id: &str, status: ThreadStatus) -> ThreadRow {
        let row = thread(id, workspace_id, status);
        self.upsert_thread(&row).unwrap();
        row
    }
}
