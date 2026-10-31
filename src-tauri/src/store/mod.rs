mod migrate;
pub mod types;

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::error::{Error, Result};
use types::{
    ApprovalRow, FindingRow, PersistedSnapshot, PrRow, RepoRecord, ScratchRow, SettingRow,
    ThreadEventRow, ThreadRow, WorkspaceRow,
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

    #[cfg(test)]
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
                "INSERT INTO repo_config (repo_id, setup_commands, run_command, single_instance)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(repo_id) DO UPDATE SET setup_commands = excluded.setup_commands,
                    run_command = excluded.run_command, single_instance = excluded.single_instance",
                rusqlite::params![
                    repo.id,
                    repo.setup_commands,
                    repo.run_command,
                    repo.single_instance
                ],
            )?;
            Ok(())
        })
    }

    /// Removes a repo with its torn-down workspaces and cached PRs, which reference it.
    /// Callers refuse while the repo still has active workspaces.
    pub fn delete_repo(&self, repo_id: &str) -> Result<()> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            // Threads only name their owner, so they go first; their events and approvals cascade.
            tx.execute(
                "DELETE FROM threads WHERE workspace_id IN
                     (SELECT id FROM workspaces WHERE repo_id = ?1)",
                [repo_id],
            )?;
            tx.execute("DELETE FROM workspaces WHERE repo_id = ?1", [repo_id])?;
            tx.execute("DELETE FROM pr_cache WHERE repo_id = ?1", [repo_id])?;
            tx.execute("DELETE FROM repos WHERE id = ?1", [repo_id])?;
            tx.commit()?;
            Ok(())
        })
    }

    pub fn count_active_workspaces_for_repo(&self, repo_id: &str) -> Result<usize> {
        self.with_conn(|conn| {
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM workspaces WHERE repo_id = ?1 AND archived_at IS NULL",
                rusqlite::params![repo_id],
                |row| row.get(0),
            )?;
            Ok(count as usize)
        })
    }

    pub fn repo_count(&self) -> Result<usize> {
        self.with_conn(|conn| {
            let count: i64 = conn.query_row("SELECT COUNT(*) FROM repos", [], |row| row.get(0))?;
            Ok(count as usize)
        })
    }

    pub fn upsert_workspace(&self, workspace: &WorkspaceRow) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO workspaces (
                    id, repo_id, name, branch, worktree_path, status,
                    summary, summary_at, summary_source, kind, pr_number, modified_files
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                 ON CONFLICT(id) DO UPDATE SET name = excluded.name, branch = excluded.branch,
                    worktree_path = excluded.worktree_path, status = excluded.status",
                rusqlite::params![
                    workspace.id,
                    workspace.repo_id,
                    workspace.name,
                    workspace.branch,
                    workspace.worktree_path,
                    workspace.status,
                    workspace.summary,
                    workspace.summary_at,
                    workspace.summary_source,
                    workspace.kind,
                    workspace.pr_number,
                    workspace.modified_files,
                ],
            )?;
            Ok(())
        })
    }

    pub fn set_workspace_name(&self, workspace_id: &str, name: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE workspaces SET name = ?1 WHERE id = ?2",
                rusqlite::params![name, workspace_id],
            )?;
            Ok(())
        })
    }

    pub fn set_workspace_branch(&self, workspace_id: &str, branch: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE workspaces SET branch = ?1 WHERE id = ?2",
                rusqlite::params![branch, workspace_id],
            )?;
            Ok(())
        })
    }

    pub fn set_workspace_pr_number(
        &self,
        workspace_id: &str,
        pr_number: Option<i64>,
    ) -> Result<()> {
        self.set_workspace_pr(workspace_id, pr_number, None)
    }

    pub fn set_workspace_pr(
        &self,
        workspace_id: &str,
        pr_number: Option<i64>,
        pr_html_url: Option<&str>,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE workspaces SET pr_number = ?1, pr_html_url = ?2 WHERE id = ?3",
                rusqlite::params![pr_number, pr_html_url, workspace_id],
            )?;
            Ok(())
        })
    }

    pub fn set_workspace_summary(
        &self,
        workspace_id: &str,
        summary: &str,
        source: &str,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE workspaces SET summary = ?1, summary_at = datetime('now'), summary_source = ?2
                 WHERE id = ?3",
                rusqlite::params![summary, source, workspace_id],
            )?;
            Ok(())
        })
    }

    pub fn archive_workspace(&self, workspace_id: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE workspaces
                 SET archived_at = datetime('now'), status = 'archived', worktree_path = ''
                 WHERE id = ?1",
                rusqlite::params![workspace_id],
            )?;
            Ok(())
        })
    }

    /// A repo's torn down workspaces with their threads, most recently closed first.
    pub fn closed_threads(
        &self,
        repo_id: &str,
        limit: i64,
    ) -> Result<Vec<(ThreadRow, WorkspaceRow)>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, repo_id, name, branch, worktree_path, status, created_at,
                        summary, summary_at, summary_source, kind, pr_number, pr_html_url,
                        modified_files, archived_at
                 FROM workspaces WHERE repo_id = ?1 AND archived_at IS NOT NULL
                 ORDER BY archived_at DESC, rowid DESC LIMIT ?2",
            )?;
            let workspaces = stmt
                .query_map(rusqlite::params![repo_id, limit], row_to_workspace)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|error| Error::Store(error.to_string()))?;
            let mut stmt = conn.prepare(&format!("{THREAD_COLUMNS} WHERE workspace_id = ?1"))?;
            let mut closed = Vec::new();
            for workspace in workspaces {
                let threads = stmt
                    .query_map([&workspace.id], row_to_thread)?
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(|error| Error::Store(error.to_string()))?;
                closed.extend(threads.into_iter().map(|row| (row, workspace.clone())));
            }
            Ok(closed)
        })
    }

    pub fn workspace_by_id(&self, workspace_id: &str) -> Result<Option<WorkspaceRow>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, repo_id, name, branch, worktree_path, status, created_at,
                        summary, summary_at, summary_source, kind, pr_number, pr_html_url,
                        modified_files, archived_at
                 FROM workspaces WHERE id = ?1",
            )?;
            let mut rows = stmt.query([workspace_id])?;
            if let Some(row) = rows.next()? {
                Ok(Some(
                    row_to_workspace(row).map_err(|error| Error::Store(error.to_string()))?,
                ))
            } else {
                Ok(None)
            }
        })
    }

    /// Workspaces that haven't been archived, without the rest of the snapshot.
    pub fn live_workspaces(&self) -> Result<Vec<WorkspaceRow>> {
        self.with_conn(|conn| {
            query_all(
                conn,
                "SELECT id, repo_id, name, branch, worktree_path, status, created_at,
                        summary, summary_at, summary_source, kind, pr_number, pr_html_url,
                        modified_files, archived_at
                 FROM workspaces WHERE archived_at IS NULL",
                row_to_workspace,
            )
        })
    }

    pub fn set_workspace_status(&self, workspace_id: &str, status: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE workspaces SET status = ?1 WHERE id = ?2",
                rusqlite::params![status, workspace_id],
            )?;
            Ok(())
        })
    }

    /// Approvals nothing can answer any more: the turn that asked has ended. Left pending,
    /// they would hold the thread in "needs attention" for good.
    pub fn deny_stale_approvals(&self, thread_id: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE approvals SET status = 'denied'
                 WHERE status = 'pending' AND thread_id = ?1",
                [thread_id],
            )?;
            Ok(())
        })
    }

    /// No agent survives the app closing, so a turn in flight then is over. Provisioning is
    /// left alone: it has its own retry.
    pub fn reset_interrupted_turns(&self) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute_batch(
                "UPDATE approvals SET status = 'denied' WHERE status = 'pending';
                 UPDATE threads SET status = 'idle'
                 WHERE status IN ('running', 'waiting', 'paused');
                 UPDATE workspaces SET status = 'idle'
                 WHERE status IN ('running', 'waiting');",
            )?;
            Ok(())
        })
    }

    pub fn mark_thread_idle(&self, thread_id: &str) -> Result<()> {
        self.set_thread_status(thread_id, "idle")?;
        let snapshot = self.snapshot()?;
        let Some(thread) = snapshot.threads.iter().find(|row| row.id == thread_id) else {
            return Ok(());
        };
        let siblings = || {
            snapshot
                .threads
                .iter()
                .filter(|row| row.workspace_id == thread.workspace_id)
        };
        if !siblings().any(|row| row.status == "running") {
            // Another thread's plan still waiting on the user keeps the workspace waiting.
            let waiting = siblings().any(|row| row.status == "waiting");
            self.set_workspace_status(
                &thread.workspace_id,
                if waiting { "waiting" } else { "idle" },
            )?;
        }
        Ok(())
    }

    pub fn set_thread_status(&self, thread_id: &str, status: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE threads SET status = ?1 WHERE id = ?2",
                rusqlite::params![status, thread_id],
            )?;
            Ok(())
        })
    }

    pub fn set_thread_session(&self, thread_id: &str, session_id: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE threads SET session_id = ?1, transcript_readonly = 0 WHERE id = ?2",
                rusqlite::params![session_id, thread_id],
            )?;
            Ok(())
        })
    }

    /// Forgets the engine session so the next prompt starts a fresh one; the transcript stays.
    pub fn clear_thread_session(&self, thread_id: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE threads SET session_id = NULL, used_tokens = NULL, context_size = NULL
                 WHERE id = ?1",
                [thread_id],
            )?;
            Ok(())
        })
    }

    pub fn thread_session(&self, thread_id: &str) -> Result<Option<String>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT session_id FROM threads WHERE id = ?1")?;
            let mut rows = stmt.query([thread_id])?;
            if let Some(row) = rows.next()? {
                Ok(row.get(0)?)
            } else {
                Ok(None)
            }
        })
    }

    /// The model the user picked for the thread; `None` means the engine's default.
    pub fn thread_model(&self, thread_id: &str) -> Result<Option<String>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT model FROM threads WHERE id = ?1")?;
            let mut rows = stmt.query([thread_id])?;
            if let Some(row) = rows.next()? {
                Ok(row.get(0)?)
            } else {
                Ok(None)
            }
        })
    }

    pub fn set_thread_model(&self, thread_id: &str, model: Option<&str>) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE threads SET model = ?1 WHERE id = ?2",
                rusqlite::params![model, thread_id],
            )?;
            Ok(())
        })
    }

    pub fn set_thread_usage(
        &self,
        thread_id: &str,
        used_tokens: u64,
        context_size: u64,
        cost_usd: Option<f64>,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE threads SET used_tokens = ?1, context_size = ?2, cost_usd = ?3 WHERE id = ?4",
                rusqlite::params![used_tokens as i64, context_size as i64, cost_usd, thread_id],
            )?;
            Ok(())
        })
    }

    pub fn transcript_summary(&self, thread_id: &str, limit: usize) -> Result<String> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT kind, payload FROM thread_events WHERE thread_id = ?1 ORDER BY seq DESC LIMIT ?2",
            )?;
            let rows = stmt.query_map(rusqlite::params![thread_id, limit as i64], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            let mut lines = Vec::new();
            for row in rows {
                let (kind, payload) = row?;
                lines.push(format!("{kind}: {payload}"));
            }
            lines.reverse();
            Ok(lines.join("\n"))
        })
    }

    pub fn mark_finding_fixed(&self, finding_id: &str, commit_sha: &str) -> Result<()> {
        self.with_conn(|conn| {
            let changed = conn.execute(
                "UPDATE findings SET status = 'fixed', commit_sha = ?1 WHERE id = ?2",
                rusqlite::params![commit_sha, finding_id],
            )?;
            if changed == 0 {
                return Err(Error::Store(format!("unknown finding {finding_id}")));
            }
            Ok(())
        })
    }

    pub fn mark_findings_sent(&self, finding_ids: &[String], thread_id: &str) -> Result<()> {
        self.with_conn(|conn| {
            for id in finding_ids {
                let changed = conn.execute(
                    "UPDATE findings SET status = 'sent', sent_to_thread_id = ?1 WHERE id = ?2 AND status = 'open'",
                    rusqlite::params![thread_id, id],
                )?;
                if changed == 0 {
                    return Err(Error::Store(format!(
                        "finding {id} is not open or does not exist"
                    )));
                }
            }
            Ok(())
        })
    }

    /// Dismisses open findings in a workspace, or restores dismissed ones to open. Findings
    /// already sent or fixed are left alone. Returns how many changed.
    pub fn set_findings_dismissed(
        &self,
        workspace_id: &str,
        finding_ids: &[String],
        dismissed: bool,
    ) -> Result<usize> {
        let (from, to) = if dismissed {
            ("open", "dismissed")
        } else {
            ("dismissed", "open")
        };
        self.with_conn(|conn| {
            let mut changed = 0;
            for id in finding_ids {
                changed += conn.execute(
                    "UPDATE findings SET status = ?1 WHERE id = ?2 AND workspace_id = ?3 AND status = ?4",
                    rusqlite::params![to, id, workspace_id, from],
                )?;
            }
            Ok(changed)
        })
    }

    pub fn finding_by_id(&self, finding_id: &str) -> Result<Option<FindingRow>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, workspace_id, severity, title, file, line, explanation, status, commit_sha, sent_to_thread_id
                 FROM findings WHERE id = ?1",
            )?;
            let mut rows = stmt.query([finding_id])?;
            if let Some(row) = rows.next()? {
                Ok(Some(FindingRow {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    severity: row.get(2)?,
                    title: row.get(3)?,
                    file: row.get(4)?,
                    line: row.get(5)?,
                    explanation: row.get(6)?,
                    status: row.get(7)?,
                    commit_sha: row.get(8)?,
                    sent_to_thread_id: row.get(9)?,
                }))
            } else {
                Ok(None)
            }
        })
    }

    pub fn upsert_thread(&self, thread: &ThreadRow) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO threads (id, workspace_id, title, engine, session_id, status,
                    used_tokens, context_size, cost_usd, transcript_readonly, retired,
                    parent_thread_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                 ON CONFLICT(id) DO UPDATE SET title = excluded.title, engine = excluded.engine,
                    session_id = excluded.session_id, status = excluded.status,
                    used_tokens = excluded.used_tokens, context_size = excluded.context_size,
                    cost_usd = excluded.cost_usd, transcript_readonly = excluded.transcript_readonly,
                    retired = excluded.retired",
                rusqlite::params![
                    thread.id,
                    thread.workspace_id,
                    thread.title,
                    thread.engine,
                    thread.session_id,
                    thread.status,
                    thread.used_tokens,
                    thread.context_size,
                    thread.cost_usd,
                    thread.transcript_readonly as i64,
                    thread.retired as i64,
                    thread.parent_thread_id,
                ],
            )?;
            Ok(())
        })
    }

    /// Any thread, including a scratch's. `snapshot().threads` only lists workspace threads.
    pub fn thread_by_id(&self, thread_id: &str) -> Result<Option<ThreadRow>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(&format!("{THREAD_COLUMNS} WHERE id = ?1"))?;
            let mut rows = stmt.query([thread_id])?;
            match rows.next()? {
                Some(row) => Ok(Some(row_to_thread(row)?)),
                None => Ok(None),
            }
        })
    }

    #[cfg(test)]
    pub fn insert_scratch(&self, scratch: &ScratchRow) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO scratches (id, repo_id, title, thread_id) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![
                    scratch.id,
                    scratch.repo_id,
                    scratch.title,
                    scratch.thread_id
                ],
            )?;
            Ok(())
        })
    }

    pub fn scratch_by_id(&self, scratch_id: &str) -> Result<Option<ScratchRow>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(&format!("{SCRATCH_COLUMNS} WHERE s.id = ?1"))?;
            let mut rows = stmt.query([scratch_id])?;
            match rows.next()? {
                Some(row) => Ok(Some(row_to_scratch(row)?)),
                None => Ok(None),
            }
        })
    }

    pub fn scratch_for_thread(&self, thread_id: &str) -> Result<Option<ScratchRow>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(&format!("{SCRATCH_COLUMNS} WHERE s.thread_id = ?1"))?;
            let mut rows = stmt.query([thread_id])?;
            match rows.next()? {
                Some(row) => Ok(Some(row_to_scratch(row)?)),
                None => Ok(None),
            }
        })
    }

    /// Removes the scratch and its thread; events and approvals cascade with the thread.
    pub fn delete_scratch(&self, scratch_id: &str) -> Result<()> {
        self.with_conn(|conn| {
            let thread_id: Option<String> = conn
                .query_row(
                    "SELECT thread_id FROM scratches WHERE id = ?1",
                    [scratch_id],
                    |row| row.get(0),
                )
                .ok();
            conn.execute("DELETE FROM scratches WHERE id = ?1", [scratch_id])?;
            if let Some(thread_id) = thread_id {
                conn.execute("DELETE FROM threads WHERE id = ?1", [thread_id])?;
            }
            Ok(())
        })
    }

    /// Everything the agent said since the user's last message, with streamed chunks joined.
    pub fn last_agent_reply(&self, thread_id: &str) -> Result<String> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT payload FROM thread_events WHERE thread_id = ?1 AND kind = 'message' ORDER BY seq",
            )?;
            let payloads = stmt
                .query_map([thread_id], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let mut reply = String::new();
            for payload in payloads {
                let Ok(value) = serde_json::from_str::<serde_json::Value>(&payload) else {
                    continue;
                };
                let text = value.get("text").and_then(|text| text.as_str()).unwrap_or("");
                match value.get("role").and_then(|role| role.as_str()) {
                    Some("user") => reply.clear(),
                    Some("agent") => reply.push_str(text),
                    _ => {}
                }
            }
            Ok(reply)
        })
    }

    /// Where the thread's conversation last moved, so a turn is only reported once.
    pub fn last_message_seq(&self, thread_id: &str) -> Result<i64> {
        self.with_conn(|conn| {
            Ok(conn.query_row(
                "SELECT COALESCE(MAX(seq), 0) FROM thread_events
                 WHERE thread_id = ?1 AND kind = 'message'",
                [thread_id],
                |row| row.get(0),
            )?)
        })
    }

    /// A page of the transcript after `since`, oldest first, as `(seq, kind, payload)`.
    pub fn thread_events_after(
        &self,
        thread_id: &str,
        since: i64,
        limit: i64,
    ) -> Result<Vec<(i64, String, String)>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT seq, kind, payload FROM thread_events
                 WHERE thread_id = ?1 AND seq > ?2 ORDER BY seq LIMIT ?3",
            )?;
            let rows = stmt
                .query_map(rusqlite::params![thread_id, since, limit], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            Ok(rows)
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

    pub fn get_approval(&self, id: &str) -> Result<Option<ApprovalRow>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, thread_id, status, tool, payload FROM approvals WHERE id = ?1",
            )?;
            let mut rows = stmt.query([id])?;
            if let Some(row) = rows.next()? {
                Ok(Some(ApprovalRow {
                    id: row.get(0)?,
                    thread_id: row.get(1)?,
                    status: row.get(2)?,
                    tool: row.get(3)?,
                    payload: row.get(4)?,
                }))
            } else {
                Ok(None)
            }
        })
    }

    pub fn pending_approvals_for_thread(&self, thread_id: &str) -> Result<Vec<ApprovalRow>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, thread_id, status, tool, payload FROM approvals
                 WHERE thread_id = ?1 AND status = 'pending' ORDER BY rowid",
            )?;
            let rows = stmt.query_map([thread_id], |row| {
                Ok(ApprovalRow {
                    id: row.get(0)?,
                    thread_id: row.get(1)?,
                    status: row.get(2)?,
                    tool: row.get(3)?,
                    payload: row.get(4)?,
                })
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| Error::Store(error.to_string()))
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

    pub fn delete_findings_for_workspace(&self, workspace_id: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "DELETE FROM findings WHERE workspace_id = ?1",
                [workspace_id],
            )?;
            Ok(())
        })
    }

    pub fn upsert_finding(&self, finding: &FindingRow) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO findings (id, workspace_id, severity, title, file, line, explanation, status, commit_sha, sent_to_thread_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(id) DO UPDATE SET severity = excluded.severity, title = excluded.title,
                    explanation = excluded.explanation, file = excluded.file, line = excluded.line,
                    status = excluded.status, commit_sha = excluded.commit_sha,
                    sent_to_thread_id = excluded.sent_to_thread_id",
                rusqlite::params![
                    finding.id,
                    finding.workspace_id,
                    finding.severity,
                    finding.title,
                    finding.file,
                    finding.line,
                    finding.explanation,
                    finding.status,
                    finding.commit_sha,
                    finding.sent_to_thread_id
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

    pub fn replace_pull_requests(&self, rows: &[PrRow]) -> Result<()> {
        self.with_conn(|conn| {
            // One transaction so a concurrent snapshot never sees an empty table mid-sync.
            let tx = conn.unchecked_transaction()?;
            tx.execute("DELETE FROM pr_cache", [])?;
            for pr in rows {
                tx.execute(
                    "INSERT INTO pr_cache (id, repo_id, number, title, payload)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![pr.id, pr.repo_id, pr.number, pr.title, pr.payload],
                )?;
            }
            tx.commit()?;
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
                            COALESCE(c.setup_commands, ''), c.run_command,
                            COALESCE(c.single_instance, 0) != 0
                     FROM repos r LEFT JOIN repo_config c ON c.repo_id = r.id",
                    |row| {
                        Ok(RepoRecord {
                            id: row.get(0)?,
                            path: row.get(1)?,
                            name: row.get(2)?,
                            default_branch: row.get(3)?,
                            setup_commands: row.get(4)?,
                            run_command: row.get(5)?,
                            single_instance: row.get(6)?,
                        })
                    },
                )?,
                workspaces: query_all(
                    conn,
                    "SELECT id, repo_id, name, branch, worktree_path, status, created_at,
                            summary, summary_at, summary_source, kind, pr_number, pr_html_url,
                            modified_files, archived_at
                     FROM workspaces WHERE archived_at IS NULL",
                    row_to_workspace,
                )?,
                threads: query_all(
                    conn,
                    &format!(
                        "{THREAD_COLUMNS}
                         WHERE closed_at IS NULL AND workspace_id IN (
                            SELECT id FROM workspaces WHERE archived_at IS NULL
                         )
                         ORDER BY created_at, rowid"
                    ),
                    row_to_thread,
                )?,
                scratches: query_all(
                    conn,
                    &format!("{SCRATCH_COLUMNS} ORDER BY s.created_at DESC, s.rowid DESC"),
                    row_to_scratch,
                )?,
                timeline: query_all(
                    conn,
                    "SELECT id, thread_id, seq, kind, payload, created_at FROM thread_events
                     WHERE thread_id NOT IN (SELECT id FROM threads WHERE closed_at IS NOT NULL)
                     ORDER BY thread_id, seq",
                    |row| {
                        Ok(ThreadEventRow {
                            id: row.get(0)?,
                            thread_id: row.get(1)?,
                            seq: row.get(2)?,
                            kind: row.get(3)?,
                            payload: row.get(4)?,
                            created_at: row.get(5)?,
                        })
                    },
                )?,
                approvals: query_all(
                    conn,
                    "SELECT id, thread_id, status, tool, payload FROM approvals ORDER BY rowid",
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
                    "SELECT id, workspace_id, severity, title, file, line, explanation, status, commit_sha, sent_to_thread_id
                     FROM findings
                     WHERE workspace_id IN (
                        SELECT id FROM workspaces WHERE archived_at IS NULL
                     )",
                    |row| {
                        Ok(FindingRow {
                            id: row.get(0)?,
                            workspace_id: row.get(1)?,
                            severity: row.get(2)?,
                            title: row.get(3)?,
                            file: row.get(4)?,
                            line: row.get(5)?,
                            explanation: row.get(6)?,
                            status: row.get(7)?,
                            commit_sha: row.get(8)?,
                            sent_to_thread_id: row.get(9)?,
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

const THREAD_COLUMNS: &str = "SELECT id, workspace_id, title, engine, session_id, status,
        used_tokens, context_size, cost_usd, transcript_readonly, retired, parent_thread_id
     FROM threads";

fn row_to_thread(row: &rusqlite::Row<'_>) -> rusqlite::Result<ThreadRow> {
    Ok(ThreadRow {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        title: row.get(2)?,
        engine: row.get(3)?,
        session_id: row.get(4)?,
        status: row.get(5)?,
        used_tokens: row.get(6)?,
        context_size: row.get(7)?,
        cost_usd: row.get(8)?,
        transcript_readonly: row.get::<_, i64>(9)? != 0,
        retired: row.get::<_, i64>(10)? != 0,
        parent_thread_id: row.get(11)?,
    })
}

const SCRATCH_COLUMNS: &str = "SELECT s.id, s.repo_id, s.title, s.thread_id, t.engine, t.status,
        s.created_at
     FROM scratches s JOIN threads t ON t.id = s.thread_id";

fn row_to_scratch(row: &rusqlite::Row<'_>) -> rusqlite::Result<ScratchRow> {
    Ok(ScratchRow {
        id: row.get(0)?,
        repo_id: row.get(1)?,
        title: row.get(2)?,
        thread_id: row.get(3)?,
        engine: row.get(4)?,
        status: row.get(5)?,
        created_at: row.get(6)?,
    })
}

fn row_to_workspace(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkspaceRow> {
    Ok(WorkspaceRow {
        id: row.get(0)?,
        repo_id: row.get(1)?,
        name: row.get(2)?,
        branch: row.get(3)?,
        worktree_path: row.get(4)?,
        status: row.get(5)?,
        created_at: row.get(6)?,
        summary: row.get(7)?,
        summary_at: row.get(8)?,
        summary_source: row.get(9)?,
        kind: row.get(10)?,
        pr_number: row.get(11)?,
        pr_html_url: row.get(12)?,
        modified_files: row.get(13)?,
        archived_at: row.get(14)?,
    })
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
    fn scratch_keeps_its_thread_out_of_workspace_lists() {
        let store = Store::new();
        store.open_in_memory().unwrap();
        store
            .upsert_thread(&ThreadRow {
                id: "t1".into(),
                workspace_id: "scratch-1".into(),
                title: "Scratch".into(),
                engine: "claude".into(),
                session_id: None,
                status: "running".into(),
                used_tokens: None,
                context_size: None,
                cost_usd: None,
                transcript_readonly: false,
                retired: false,
                parent_thread_id: None,
            })
            .unwrap();
        store
            .insert_scratch(&ScratchRow {
                id: "scratch-1".into(),
                repo_id: "gone-repo".into(),
                title: "Why".into(),
                thread_id: "t1".into(),
                engine: String::new(),
                status: String::new(),
                created_at: String::new(),
            })
            .unwrap();
        store.append_event("t1", "message", "{}").unwrap();
        store.mark_thread_idle("t1").unwrap();

        let snapshot = store.snapshot().unwrap();
        assert!(snapshot.threads.is_empty());
        assert_eq!(snapshot.scratches.len(), 1);
        assert_eq!(snapshot.scratches[0].engine, "claude");
        assert_eq!(snapshot.scratches[0].status, "idle");
        assert_eq!(
            store.scratch_for_thread("t1").unwrap().unwrap().id,
            "scratch-1"
        );
        assert!(store.thread_by_id("t1").unwrap().is_some());

        store.delete_scratch("scratch-1").unwrap();
        let snapshot = store.snapshot().unwrap();
        assert!(snapshot.scratches.is_empty());
        assert!(snapshot.timeline.is_empty());
        assert!(store.thread_by_id("t1").unwrap().is_none());
    }

    #[test]
    fn deleting_a_repo_clears_its_archived_workspaces_and_prs() {
        let store = Store::new();
        store.open_in_memory().unwrap();
        for id in ["r1", "r2"] {
            store
                .upsert_repo(&RepoRecord {
                    id: id.into(),
                    path: format!("/tmp/{id}"),
                    name: id.into(),
                    default_branch: Some("main".into()),
                    setup_commands: String::new(),
                    run_command: None,
                    single_instance: false,
                })
                .unwrap();
        }
        store
            .upsert_workspace(&WorkspaceRow {
                id: "w1".into(),
                repo_id: "r1".into(),
                name: "Old".into(),
                branch: "feat".into(),
                worktree_path: "/tmp/wt".into(),
                status: "ready".into(),
                created_at: String::new(),
                summary: None,
                summary_at: None,
                summary_source: String::new(),
                kind: None,
                pr_number: None,
                pr_html_url: None,
                modified_files: 0,
                archived_at: None,
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
                used_tokens: None,
                context_size: None,
                cost_usd: None,
                transcript_readonly: false,
                retired: false,
                parent_thread_id: None,
            })
            .unwrap();
        store.append_event("t1", "message", "{}").unwrap();
        store.archive_workspace("w1").unwrap();
        for (id, repo) in [("p1", "r1"), ("p2", "r2")] {
            store
                .upsert_pr(&PrRow {
                    id: id.into(),
                    repo_id: Some(repo.into()),
                    number: 1,
                    title: "PR".into(),
                    payload: "{}".into(),
                })
                .unwrap();
        }

        store.delete_repo("r1").unwrap();

        let snapshot = store.snapshot().unwrap();
        let repos: Vec<_> = snapshot.repos.iter().map(|repo| repo.id.as_str()).collect();
        assert_eq!(repos, ["r2"]);
        assert!(store.workspace_by_id("w1").unwrap().is_none());
        assert!(store.thread_by_id("t1").unwrap().is_none());
        let prs: Vec<_> = snapshot
            .pull_requests
            .iter()
            .map(|pr| pr.id.as_str())
            .collect();
        assert_eq!(prs, ["p2"]);
    }

    #[test]
    fn last_agent_reply_joins_chunks_since_the_last_user_message() {
        let store = Store::new();
        store.open_in_memory().unwrap();
        store
            .upsert_repo(&RepoRecord {
                id: "r1".into(),
                path: "/tmp/app".into(),
                name: "app".into(),
                default_branch: Some("main".into()),
                setup_commands: String::new(),
                run_command: None,
                single_instance: false,
            })
            .unwrap();
        store
            .upsert_workspace(&WorkspaceRow {
                id: "w1".into(),
                repo_id: "r1".into(),
                name: "Review".into(),
                branch: "feat".into(),
                worktree_path: "/tmp/wt".into(),
                status: "running".into(),
                created_at: String::new(),
                summary: None,
                summary_at: None,
                summary_source: "Haiku 4.5".into(),
                kind: Some("review".into()),
                pr_number: Some(2),
                pr_html_url: None,
                modified_files: 0,
                archived_at: None,
            })
            .unwrap();
        store
            .upsert_thread(&ThreadRow {
                id: "t1".into(),
                workspace_id: "w1".into(),
                title: "Reviewer".into(),
                engine: "claude".into(),
                session_id: None,
                status: "running".into(),
                used_tokens: None,
                context_size: None,
                cost_usd: None,
                transcript_readonly: false,
                retired: false,
                parent_thread_id: None,
            })
            .unwrap();
        let events = [
            ("message", r#"{"role":"user","text":"Review #2"}"#),
            (
                "message",
                r#"{"type":"messageChunk","role":"agent","text":"Old "}"#,
            ),
            (
                "message",
                r#"{"type":"messageChunk","role":"user","text":"Again"}"#,
            ),
            (
                "message",
                r#"{"type":"messageChunk","role":"thought","text":"hmm"}"#,
            ),
            (
                "message",
                r#"{"type":"messageChunk","role":"agent","text":"Looks "}"#,
            ),
            ("tool", r#"{"type":"toolCall","title":"Read"}"#),
            (
                "message",
                r#"{"type":"messageChunk","role":"agent","text":"good."}"#,
            ),
        ];
        for (kind, payload) in events {
            store.append_event("t1", kind, payload).unwrap();
        }
        assert_eq!(store.last_agent_reply("t1").unwrap(), "Looks good.");
    }

    #[test]
    fn thread_session_and_model_survive_an_upsert() {
        let store = Store::new();
        store.open_in_memory().unwrap();
        store
            .upsert_repo(&RepoRecord {
                id: "r1".into(),
                path: "/tmp/app".into(),
                name: "app".into(),
                default_branch: Some("main".into()),
                setup_commands: String::new(),
                run_command: None,
                single_instance: false,
            })
            .unwrap();
        store
            .upsert_workspace(&WorkspaceRow {
                id: "w1".into(),
                repo_id: "r1".into(),
                name: "Login".into(),
                branch: "feat".into(),
                worktree_path: "/tmp/wt".into(),
                status: "idle".into(),
                created_at: String::new(),
                summary: None,
                summary_at: None,
                summary_source: "Haiku 4.5".into(),
                kind: None,
                pr_number: None,
                pr_html_url: None,
                modified_files: 0,
                archived_at: None,
            })
            .unwrap();
        for (id, title) in std::iter::once(("lead", "Lead")) {
            store
                .upsert_thread(&ThreadRow {
                    id: id.into(),
                    workspace_id: "w1".into(),
                    title: title.into(),
                    engine: "claude".into(),
                    session_id: None,
                    status: "running".into(),
                    used_tokens: None,
                    context_size: None,
                    cost_usd: None,
                    transcript_readonly: false,
                    retired: false,
                    parent_thread_id: None,
                })
                .unwrap();
            store.append_event(id, "message", "{}").unwrap();
        }
        store.set_thread_session("lead", "sess-1").unwrap();
        store
            .set_thread_usage("lead", 900, 1000, Some(0.5))
            .unwrap();
        store.clear_thread_session("lead").unwrap();
        assert_eq!(store.thread_session("lead").unwrap(), None);

        assert_eq!(store.thread_model("lead").unwrap(), None);
        store.set_thread_model("lead", Some("opus")).unwrap();
        let lead = store.thread_by_id("lead").unwrap().unwrap();
        store.upsert_thread(&lead).unwrap();
        assert_eq!(store.thread_model("lead").unwrap().as_deref(), Some("opus"));
        store.set_thread_model("lead", None).unwrap();
        assert_eq!(store.thread_model("lead").unwrap(), None);
        let lead = store.thread_by_id("lead").unwrap().unwrap();
        assert_eq!((lead.used_tokens, lead.context_size), (None, None));
        assert_eq!(store.snapshot().unwrap().timeline.len(), 1);
    }

    #[test]
    fn a_restart_ends_the_turns_that_were_running() {
        let store = Store::new();
        store.open_in_memory().unwrap();
        store
            .with_conn(|conn| {
                conn.execute_batch(
                    "PRAGMA foreign_keys = OFF;
                     INSERT INTO threads (id, workspace_id, title, engine, status) VALUES
                        ('busy', 'w', 'Lead', 'claude', 'running'),
                        ('setup', 'w2', 'Lead', 'claude', 'provisioning');
                     INSERT INTO approvals (id, thread_id, status, tool, payload) VALUES
                        ('stale', 'busy', 'pending', 'execute', '{}'),
                        ('done', 'busy', 'approved', 'execute', '{}');",
                )?;
                Ok(())
            })
            .unwrap();

        store.reset_interrupted_turns().unwrap();

        let snapshot = store.snapshot().unwrap();
        assert!(snapshot.approvals.iter().all(|row| row.status != "pending"));
        let status = |id: &str| store.thread_by_id(id).unwrap().unwrap().status;
        assert_eq!(status("busy"), "idle");
        assert_eq!(status("setup"), "provisioning");
    }

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
                single_instance: false,
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
                created_at: String::new(),
                summary: None,
                summary_at: None,
                summary_source: "Haiku 4.5".into(),
                kind: None,
                pr_number: None,
                pr_html_url: None,
                modified_files: 0,
                archived_at: None,
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
                used_tokens: None,
                context_size: None,
                cost_usd: None,
                transcript_readonly: false,
                retired: false,
                parent_thread_id: None,
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
                status: "open".into(),
                commit_sha: None,
                sent_to_thread_id: None,
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

        // Dismissing only touches open findings of that workspace, and restoring undoes it.
        let ids = vec!["f1".to_string()];
        assert_eq!(store.set_findings_dismissed("w2", &ids, true).unwrap(), 0);
        assert_eq!(store.set_findings_dismissed("w1", &ids, true).unwrap(), 1);
        assert_eq!(store.set_findings_dismissed("w1", &ids, true).unwrap(), 0);
        assert_eq!(
            store.finding_by_id("f1").unwrap().unwrap().status,
            "dismissed"
        );
        assert_eq!(store.set_findings_dismissed("w1", &ids, false).unwrap(), 1);
        assert_eq!(store.finding_by_id("f1").unwrap().unwrap().status, "open");

        store
            .set_workspace_summary("w1", "Working on login.", "Haiku 4.5")
            .unwrap();
        let row = store.workspace_by_id("w1").unwrap().unwrap();
        assert_eq!(row.summary.as_deref(), Some("Working on login."));
        assert!(row.summary_at.is_some());
    }
}
