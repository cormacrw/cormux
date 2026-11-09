//! What only repo management asks of the generic tables.

use crate::error::Result;
use crate::store::Store;

impl Store {
    /// Removes a repo with its torn-down workspaces and cached PRs, which reference it.
    /// Callers refuse while the repo still has active workspaces.
    pub fn delete_repo(&self, repo_id: &str) -> Result<()> {
        self.with_tx(|conn| {
            // Threads only name their owner, so they go first; their events and approvals cascade.
            conn.execute(
                "DELETE FROM threads WHERE workspace_id IN
                     (SELECT id FROM workspaces WHERE repo_id = ?1)",
                [repo_id],
            )?;
            conn.execute("DELETE FROM workspaces WHERE repo_id = ?1", [repo_id])?;
            conn.execute("DELETE FROM pr_cache WHERE repo_id = ?1", [repo_id])?;
            conn.execute("DELETE FROM repos WHERE id = ?1", [repo_id])?;
            Ok(())
        })
    }

    pub fn count_active_workspaces_for_repo(&self, repo_id: &str) -> Result<usize> {
        self.with_read(|conn| {
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM live_workspaces WHERE repo_id = ?1",
                [repo_id],
                |row| row.get(0),
            )?;
            Ok(count as usize)
        })
    }

    pub fn repo_count(&self) -> Result<usize> {
        self.with_read(|conn| {
            let count: i64 = conn.query_row("SELECT COUNT(*) FROM repos", [], |row| row.get(0))?;
            Ok(count as usize)
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{ThreadStatus, WorkspaceLifecycle};
    use crate::store::Store;
    use crate::store::fixtures::repo;

    #[test]
    fn deleting_a_repo_clears_its_archived_workspaces_and_prs() {
        let store = Store::in_memory().unwrap();
        store.seed_workspace("w1", WorkspaceLifecycle::Ready);
        store.upsert_repo(&repo("r2")).unwrap();
        store.seed_thread("t1", "w1", ThreadStatus::Idle);
        store.append_event("t1", "message", "{}").unwrap();
        store.archive_workspace("w1").unwrap();
        store
            .with_conn(|conn| {
                conn.execute_batch(
                    "INSERT INTO pr_cache (id, repo_id, number, title, payload) VALUES
                        ('p1', 'r1', 1, 'PR', '{}'), ('p2', 'r2', 1, 'PR', '{}');",
                )?;
                Ok(())
            })
            .unwrap();

        store.delete_repo("r1").unwrap();

        let repos: Vec<_> = store.repos().unwrap().into_iter().map(|r| r.id).collect();
        assert_eq!(repos, ["r2"]);
        assert!(store.workspace_by_id("w1").unwrap().is_none());
        assert!(store.thread_by_id("t1").unwrap().is_none());
        let prs: Vec<String> = store
            .with_read(|conn| {
                crate::store::query_all(conn, "SELECT id FROM pr_cache", |row| row.get(0))
            })
            .unwrap();
        assert_eq!(prs, ["p2"]);
    }
}
