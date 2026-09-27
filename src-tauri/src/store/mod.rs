mod migrate;
pub mod types;

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::error::{Error, Result};
use types::{
    ApprovalRow, FindingRow, PersistedSnapshot, PrRow, RepoRecord, SettingRow, ThreadEventRow,
    ThreadRow, WorkspaceRow,
};

/// SQLite persistence in the app data directory.
#[derive(Clone)]
pub struct Store {
    conn: Arc<Mutex<Option<Connection>>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            conn: Arc::new(Mutex::new(None)),
        }
    }

    pub fn open(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        self.configure(conn)
    }

    pub fn open_in_memory(&self) -> Result<()> {
        self.configure(Connection::open_in_memory()?)
    }

    fn configure(&self, conn: Connection) -> Result<()> {
        conn.pragma_update(None, "foreign_keys", true)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        migrate::run(&conn)?;
        *self
            .conn
            .lock()
            .map_err(|error| Error::Store(error.to_string()))? = Some(conn);
        Ok(())
    }

    fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let guard = self
            .conn
            .lock()
            .map_err(|error| Error::Store(error.to_string()))?;
        let conn = guard
            .as_ref()
            .ok_or_else(|| Error::Store("store is not open".into()))?;
        f(conn)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params![key, value],
            )?;
            Ok(())
        })
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
            let mut rows = stmt.query([key])?;
            if let Some(row) = rows.next()? {
                Ok(Some(row.get(0)?))
            } else {
                Ok(None)
            }
        })
    }

    pub fn upsert_repo(&self, repo: &RepoRecord) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO repos (id, path, name, default_branch) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET path = excluded.path, name = excluded.name,
                    default_branch = excluded.default_branch",
                rusqlite::params![repo.id, repo.path, repo.name, repo.default_branch],
            )?;
            conn.execute(
                "INSERT INTO repo_config (repo_id, setup_commands, run_command) VALUES (?1, ?2, ?3)
                 ON CONFLICT(repo_id) DO UPDATE SET setup_commands = excluded.setup_commands,
                    run_command = excluded.run_command",
                rusqlite::params![repo.id, repo.setup_commands, repo.run_command],
            )?;
            Ok(())
        })
    }

    pub fn upsert_workspace(&self, workspace: &WorkspaceRow) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO workspaces (id, repo_id, name, branch, worktree_path, status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET name = excluded.name, branch = excluded.branch,
                    worktree_path = excluded.worktree_path, status = excluded.status",
                rusqlite::params![
                    workspace.id,
                    workspace.repo_id,
                    workspace.name,
                    workspace.branch,
                    workspace.worktree_path,
                    workspace.status
                ],
            )?;
            Ok(())
        })
    }

    pub fn upsert_thread(&self, thread: &ThreadRow) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO threads (id, workspace_id, title, engine, session_id, status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET title = excluded.title, engine = excluded.engine,
                    session_id = excluded.session_id, status = excluded.status",
                rusqlite::params![
                    thread.id,
                    thread.workspace_id,
                    thread.title,
                    thread.engine,
                    thread.session_id,
                    thread.status
                ],
            )?;
            Ok(())
        })
    }

    pub fn append_event(&self, thread_id: &str, kind: &str, payload: &str) -> Result<i64> {
        self.with_conn(|conn| {
            let seq: i64 = conn.query_row(
                "SELECT COALESCE(MAX(seq), 0) + 1 FROM thread_events WHERE thread_id = ?1",
                [thread_id],
                |row| row.get(0),
            )?;
            conn.execute(
                "INSERT INTO thread_events (thread_id, seq, kind, payload) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![thread_id, seq, kind, payload],
            )?;
            Ok(seq)
        })
    }

    pub fn upsert_approval(&self, approval: &ApprovalRow) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO approvals (id, thread_id, status, tool, payload)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET status = excluded.status, payload = excluded.payload",
                rusqlite::params![
                    approval.id,
                    approval.thread_id,
                    approval.status,
                    approval.tool,
                    approval.payload
                ],
            )?;
            Ok(())
        })
    }

    pub fn upsert_finding(&self, finding: &FindingRow) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO findings (id, workspace_id, severity, title, file, line, explanation)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET severity = excluded.severity, title = excluded.title,
                    explanation = excluded.explanation",
                rusqlite::params![
                    finding.id,
                    finding.workspace_id,
                    finding.severity,
                    finding.title,
                    finding.file,
                    finding.line,
                    finding.explanation
                ],
            )?;
            Ok(())
        })
    }

    pub fn upsert_pr(&self, pr: &PrRow) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO pr_cache (id, repo_id, number, title, payload)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET title = excluded.title, payload = excluded.payload,
                    synced_at = datetime('now')",
                rusqlite::params![pr.id, pr.repo_id, pr.number, pr.title, pr.payload],
            )?;
            Ok(())
        })
    }

    pub fn snapshot(&self) -> Result<PersistedSnapshot> {
        self.with_conn(|conn| {
            Ok(PersistedSnapshot {
                settings: query_all(
                    conn,
                    "SELECT key, value FROM settings",
                    |row| {
                        Ok(SettingRow {
                            key: row.get(0)?,
                            value: row.get(1)?,
                        })
                    },
                )?,
                repos: query_all(
                    conn,
                    "SELECT r.id, r.path, r.name, r.default_branch,
                            COALESCE(c.setup_commands, ''), c.run_command
                     FROM repos r LEFT JOIN repo_config c ON c.repo_id = r.id",
                    |row| {
                        Ok(RepoRecord {
                            id: row.get(0)?,
                            path: row.get(1)?,
                            name: row.get(2)?,
                            default_branch: row.get(3)?,
                            setup_commands: row.get(4)?,
                            run_command: row.get(5)?,
                        })
                    },
                )?,
                workspaces: query_all(
                    conn,
                    "SELECT id, repo_id, name, branch, worktree_path, status FROM workspaces",
                    |row| {
                        Ok(WorkspaceRow {
                            id: row.get(0)?,
                            repo_id: row.get(1)?,
                            name: row.get(2)?,
                            branch: row.get(3)?,
                            worktree_path: row.get(4)?,
                            status: row.get(5)?,
                        })
                    },
                )?,
                threads: query_all(
                    conn,
                    "SELECT id, workspace_id, title, engine, session_id, status FROM threads",
                    |row| {
                        Ok(ThreadRow {
                            id: row.get(0)?,
                            workspace_id: row.get(1)?,
                            title: row.get(2)?,
                            engine: row.get(3)?,
                            session_id: row.get(4)?,
                            status: row.get(5)?,
                        })
                    },
                )?,
                timeline: query_all(
                    conn,
                    "SELECT id, thread_id, seq, kind, payload FROM thread_events ORDER BY thread_id, seq",
                    |row| {
                        Ok(ThreadEventRow {
                            id: row.get(0)?,
                            thread_id: row.get(1)?,
                            seq: row.get(2)?,
                            kind: row.get(3)?,
                            payload: row.get(4)?,
                        })
                    },
                )?,
                approvals: query_all(
                    conn,
                    "SELECT id, thread_id, status, tool, payload FROM approvals",
                    |row| {
                        Ok(ApprovalRow {
                            id: row.get(0)?,
                            thread_id: row.get(1)?,
                            status: row.get(2)?,
                            tool: row.get(3)?,
                            payload: row.get(4)?,
                        })
                    },
                )?,
                findings: query_all(
                    conn,
                    "SELECT id, workspace_id, severity, title, file, line, explanation FROM findings",
                    |row| {
                        Ok(FindingRow {
                            id: row.get(0)?,
                            workspace_id: row.get(1)?,
                            severity: row.get(2)?,
                            title: row.get(3)?,
                            file: row.get(4)?,
                            line: row.get(5)?,
                            explanation: row.get(6)?,
                        })
                    },
                )?,
                pull_requests: query_all(
                    conn,
                    "SELECT id, repo_id, number, title, payload FROM pr_cache",
                    |row| {
                        Ok(PrRow {
                            id: row.get(0)?,
                            repo_id: row.get(1)?,
                            number: row.get(2)?,
                            title: row.get(3)?,
                            payload: row.get(4)?,
                        })
                    },
                )?,
            })
        })
    }
}

fn query_all<T>(
    conn: &Connection,
    sql: &str,
    f: impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], f)?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_and_restores_snapshot() {
        let store = Store::new();
        store.open_in_memory().unwrap();
        store.set_setting("reduceMotion", "true").unwrap();
        store
            .upsert_repo(&RepoRecord {
                id: "r1".into(),
                path: "/tmp/app".into(),
                name: "app".into(),
                default_branch: Some("main".into()),
                setup_commands: "pnpm install".into(),
                run_command: Some("pnpm dev".into()),
            })
            .unwrap();
        store
            .upsert_workspace(&WorkspaceRow {
                id: "w1".into(),
                repo_id: "r1".into(),
                name: "Login".into(),
                branch: "feat".into(),
                worktree_path: "/tmp/wt".into(),
                status: "ready".into(),
            })
            .unwrap();
        store
            .upsert_thread(&ThreadRow {
                id: "t1".into(),
                workspace_id: "w1".into(),
                title: "Lead".into(),
                engine: "claude".into(),
                session_id: None,
                status: "idle".into(),
            })
            .unwrap();
        store
            .append_event("t1", "message", "{\"text\":\"hi\"}")
            .unwrap();
        store
            .upsert_approval(&ApprovalRow {
                id: "a1".into(),
                thread_id: "t1".into(),
                status: "pending".into(),
                tool: "edit".into(),
                payload: "{}".into(),
            })
            .unwrap();
        store
            .upsert_finding(&FindingRow {
                id: "f1".into(),
                workspace_id: "w1".into(),
                severity: "high".into(),
                title: "Leak".into(),
                file: Some("a.ts".into()),
                line: Some(10),
                explanation: "secret".into(),
            })
            .unwrap();
        store
            .upsert_pr(&PrRow {
                id: "p1".into(),
                repo_id: Some("r1".into()),
                number: 12,
                title: "Login".into(),
                payload: "{}".into(),
            })
            .unwrap();

        let snap = store.snapshot().unwrap();
        assert_eq!(snap.settings[0].value, "true");
        assert_eq!(snap.repos[0].name, "app");
        assert_eq!(snap.workspaces[0].branch, "feat");
        assert_eq!(snap.threads[0].title, "Lead");
        assert_eq!(snap.timeline[0].kind, "message");
        assert_eq!(snap.approvals[0].tool, "edit");
        assert_eq!(snap.findings[0].title, "Leak");
        assert_eq!(snap.pull_requests[0].number, 12);
    }
}
