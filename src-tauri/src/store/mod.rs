#[cfg(test)]
pub mod fixtures;
pub mod migrate;
pub mod types;

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension};

use crate::error::{Error, Result};
use crate::model::{
    ApprovalStatus, ThreadRole, ThreadStatus, WorkspaceKind, WorkspaceLifecycle,
    status_from_threads,
};
pub use migrate::RustMigration;
use types::{RepoRecord, ThreadPicks, ThreadRow, WorkspaceRow};

const STATEMENT_CACHE: usize = 64;

#[derive(Clone)]
pub struct Store {
    conn: Arc<Mutex<Connection>>,
    /// WAL lets reads run beside the writer, so a long read can't hold up the engine pumps.
    /// In-memory stores have no second connection and read through `conn`.
    reader: Option<Arc<Mutex<Connection>>>,
}

impl Store {
    /// Migrates with the SQL files and `rust` interleaved in version order.
    pub fn open(path: &Path, rust: &[RustMigration]) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = configure(Connection::open(path)?, rust)?;
        // After the migrations, which it mustn't race.
        let reader = Connection::open(path)?;
        tune(&reader)?;
        reader.pragma_update(None, "query_only", true)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            reader: Some(Arc::new(Mutex::new(reader))),
        })
    }

    #[cfg(test)]
    pub fn in_memory() -> Result<Self> {
        Ok(Self {
            conn: Arc::new(Mutex::new(configure(Connection::open_in_memory()?, &[])?)),
            reader: None,
        })
    }

    pub(crate) fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| Error::Store(error.to_string()))?;
        f(&conn)
    }

    /// Like `with_conn`, inside a transaction that commits only if `f` succeeds, so a write
    /// of several statements lands whole or not at all.
    pub(crate) fn with_tx<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            let value = f(&tx)?;
            tx.commit()?;
            Ok(value)
        })
    }

    pub(crate) fn with_read<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        match &self.reader {
            Some(reader) => {
                let conn = reader
                    .lock()
                    .map_err(|error| Error::Store(error.to_string()))?;
                f(&conn)
            }
            None => self.with_conn(f),
        }
    }

    /// Runs store work off the async runtime, for reads too big to do on a worker.
    pub async fn blocking<T: Send + 'static>(
        &self,
        f: impl FnOnce(&Store) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || f(&store)).await?
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

    pub fn set_settings(&self, rows: &[(&str, String)]) -> Result<()> {
        self.with_tx(|conn| {
            let mut stmt = conn.prepare_cached(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            )?;
            for (key, value) in rows {
                stmt.execute(rusqlite::params![key, value])?;
            }
            Ok(())
        })
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        self.with_read(|conn| {
            Ok(conn
                .prepare_cached("SELECT value FROM settings WHERE key = ?1")?
                .query_row([key], |row| row.get(0))
                .optional()?)
        })
    }

    pub fn repos(&self) -> Result<Vec<RepoRecord>> {
        self.with_read(repos_in)
    }

    pub fn repo_by_id(&self, repo_id: &str) -> Result<Option<RepoRecord>> {
        self.with_read(|conn| {
            Ok(conn
                .prepare_cached(&format!("{REPO_COLUMNS} WHERE r.id = ?1"))?
                .query_row([repo_id], row_to_repo)
                .optional()?)
        })
    }

    pub fn repo(&self, repo_id: &str) -> Result<RepoRecord> {
        self.repo_by_id(repo_id)?
            .ok_or_else(|| Error::unknown_repo(repo_id))
    }

    pub fn upsert_repo(&self, repo: &RepoRecord) -> Result<()> {
        self.with_tx(|conn| {
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

    #[cfg(test)]
    pub fn insert_workspace(&self, workspace: &WorkspaceRow) -> Result<()> {
        self.with_conn(|conn| write_workspace(conn, workspace))
    }

    pub fn insert_workspace_with_thread(
        &self,
        workspace: &WorkspaceRow,
        thread: &ThreadRow,
        picks: &ThreadPicks,
    ) -> Result<()> {
        self.with_tx(|conn| {
            write_workspace(conn, workspace)?;
            write_thread(conn, thread)?;
            conn.execute(
                "UPDATE threads SET model = ?1, effort = ?2 WHERE id = ?3",
                rusqlite::params![picks.model, picks.effort, thread.id],
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

    pub fn set_workspace_base(&self, workspace_id: &str, base: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE workspaces SET base = ?1 WHERE id = ?2",
                rusqlite::params![base, workspace_id],
            )?;
            Ok(())
        })
    }

    pub fn set_workspace_checkout(
        &self,
        workspace_id: &str,
        branch: &str,
        worktree_path: &str,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE workspaces SET branch = ?1, worktree_path = ?2 WHERE id = ?3",
                rusqlite::params![branch, worktree_path, workspace_id],
            )?;
            Ok(())
        })
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

    pub fn set_workspace_run_command(
        &self,
        workspace_id: &str,
        command: Option<&str>,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE workspaces SET run_command = ?1 WHERE id = ?2",
                rusqlite::params![command, workspace_id],
            )?;
            Ok(())
        })
    }

    pub fn set_workspace_summary(&self, workspace_id: &str, summary: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE workspaces SET summary = ?1, summary_at = datetime('now') WHERE id = ?2",
                rusqlite::params![summary, workspace_id],
            )?;
            Ok(())
        })
    }

    pub fn archive_workspace(&self, workspace_id: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE workspaces SET archived_at = datetime('now'), worktree_path = ''
                 WHERE id = ?1",
                rusqlite::params![workspace_id],
            )?;
            Ok(())
        })
    }

    pub fn workspace_by_id(&self, workspace_id: &str) -> Result<Option<WorkspaceRow>> {
        self.with_read(|conn| {
            Ok(conn
                .prepare_cached(&select_workspaces("workspaces", "WHERE w.id = ?1"))?
                .query_row([workspace_id], row_to_workspace)
                .optional()?)
        })
    }

    pub fn live_workspaces(&self) -> Result<Vec<WorkspaceRow>> {
        self.with_read(live_workspaces_in)
    }

    pub fn workspace_threads(&self) -> Result<Vec<ThreadRow>> {
        self.with_read(workspace_threads_in)
    }

    /// Runs `f` in one read transaction, so a write landing midway can't hand back rows that
    /// disagree (a thread whose workspace isn't in the same read).
    pub(crate) fn read_consistent<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        self.with_read(|conn| {
            let tx = conn.unchecked_transaction()?;
            f(&tx)
        })
    }

    pub fn threads_for_workspace(&self, workspace_id: &str) -> Result<Vec<ThreadRow>> {
        self.with_read(|conn| {
            let mut stmt = conn.prepare_cached(&select_threads(
                "threads",
                "WHERE t.workspace_id = ?1 AND t.closed_at IS NULL ORDER BY t.created_at, t.rowid",
            ))?;
            let rows = stmt.query_map([workspace_id], row_to_thread)?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }

    /// The workspace's own thread, its Lead or Reviewer: the one git steps are recorded in.
    pub fn own_thread(&self, workspace_id: &str) -> Result<Option<ThreadRow>> {
        self.with_read(|conn| {
            Ok(conn
                .prepare_cached(&select_threads(
                    "threads",
                    "WHERE t.workspace_id = ?1 AND t.role IS NOT NULL AND t.closed_at IS NULL
                     ORDER BY t.created_at, t.rowid LIMIT 1",
                ))?
                .query_row([workspace_id], row_to_thread)
                .optional()?)
        })
    }

    pub fn set_workspace_status(
        &self,
        workspace_id: &str,
        status: WorkspaceLifecycle,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE workspaces SET status = ?1 WHERE id = ?2",
                rusqlite::params![status.as_str(), workspace_id],
            )?;
            Ok(())
        })
    }

    /// No agent survives the app closing, so a turn in flight then is over. Provisioning is
    /// left alone: it has its own retry.
    pub fn reset_interrupted_turns(&self) -> Result<()> {
        self.with_tx(|conn| {
            conn.execute(
                "UPDATE approvals SET status = ?1 WHERE status = ?2",
                [
                    ApprovalStatus::Denied.as_str(),
                    ApprovalStatus::Pending.as_str(),
                ],
            )?;
            conn.execute(
                "UPDATE threads SET status = ?1 WHERE status IN (?2, ?3)",
                [
                    ThreadStatus::Idle.as_str(),
                    ThreadStatus::Running.as_str(),
                    ThreadStatus::Paused.as_str(),
                ],
            )?;
            conn.execute(
                "UPDATE workspaces SET status = ?1 WHERE status = ?2",
                [
                    WorkspaceLifecycle::Idle.as_str(),
                    WorkspaceLifecycle::Running.as_str(),
                ],
            )?;
            Ok(())
        })
    }

    /// Writes the status and re-derives the workspace's from its threads (see
    /// `status_from_threads`), so the two can't disagree. An archived workspace stays as it is.
    pub fn set_thread_status(&self, thread_id: &str, status: ThreadStatus) -> Result<()> {
        self.with_tx(|conn| {
            conn.prepare_cached("UPDATE threads SET status = ?1 WHERE id = ?2")?
                .execute(rusqlite::params![status.as_str(), thread_id])?;
            let workspace = conn
                .prepare_cached(
                    "SELECT w.id, w.status FROM workspaces w
                     JOIN threads t ON t.workspace_id = w.id
                     WHERE t.id = ?1 AND w.archived_at IS NULL",
                )?
                .query_map([thread_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .next()
                .transpose()?;
            let Some((workspace_id, current)) = workspace else {
                return Ok(());
            };
            let threads = conn
                .prepare_cached(
                    "SELECT status FROM threads WHERE workspace_id = ?1 AND closed_at IS NULL",
                )?
                .query_map([&workspace_id], |row| row.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let derived = status_from_threads(
                WorkspaceLifecycle::parse(&current),
                threads.iter().map(|raw| ThreadStatus::parse(raw)),
            );
            if derived.as_str() != current {
                conn.prepare_cached("UPDATE workspaces SET status = ?1 WHERE id = ?2")?
                    .execute(rusqlite::params![derived.as_str(), workspace_id])?;
            }
            Ok(())
        })
    }

    pub fn set_thread_session(&self, thread_id: &str, session_id: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE threads SET session_id = ?1 WHERE id = ?2",
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
        self.with_read(|conn| {
            let mut stmt = conn.prepare_cached("SELECT session_id FROM threads WHERE id = ?1")?;
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
        self.with_read(|conn| {
            let mut stmt = conn.prepare_cached("SELECT model FROM threads WHERE id = ?1")?;
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

    /// The effort level the user picked for the thread; `None` means the engine's default.
    pub fn thread_effort(&self, thread_id: &str) -> Result<Option<String>> {
        self.with_read(|conn| {
            let mut stmt = conn.prepare_cached("SELECT effort FROM threads WHERE id = ?1")?;
            let mut rows = stmt.query([thread_id])?;
            if let Some(row) = rows.next()? {
                Ok(row.get(0)?)
            } else {
                Ok(None)
            }
        })
    }

    pub fn set_thread_effort(&self, thread_id: &str, effort: Option<&str>) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE threads SET effort = ?1 WHERE id = ?2",
                rusqlite::params![effort, thread_id],
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

    #[cfg(test)]
    pub fn upsert_thread(&self, thread: &ThreadRow) -> Result<()> {
        self.with_conn(|conn| write_thread(conn, thread))
    }

    /// Any thread, including a scratch's.
    pub fn thread_by_id(&self, thread_id: &str) -> Result<Option<ThreadRow>> {
        self.with_read(|conn| {
            Ok(conn
                .prepare_cached(&select_threads("threads", "WHERE t.id = ?1"))?
                .query_row([thread_id], row_to_thread)
                .optional()?)
        })
    }
}

fn write_workspace(conn: &Connection, workspace: &WorkspaceRow) -> Result<()> {
    conn.execute(
        "INSERT INTO workspaces (
            id, repo_id, name, branch, base, worktree_path, status,
            summary, summary_at, kind, pr_number, pr_html_url, run_command
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        rusqlite::params![
            workspace.id,
            workspace.repo_id,
            workspace.name,
            workspace.branch,
            workspace.base,
            workspace.worktree_path,
            workspace.status.as_str(),
            workspace.summary,
            workspace.summary_at,
            workspace.kind.as_str(),
            workspace.pr_number,
            workspace.pr_html_url,
            workspace.run_command,
        ],
    )?;
    Ok(())
}

fn write_thread(conn: &Connection, thread: &ThreadRow) -> Result<()> {
    conn.execute(
        "INSERT INTO threads (id, workspace_id, title, engine, session_id, status,
            used_tokens, context_size, cost_usd, retired, parent_thread_id, role)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(id) DO UPDATE SET title = excluded.title, engine = excluded.engine,
            session_id = excluded.session_id, status = excluded.status,
            used_tokens = excluded.used_tokens, context_size = excluded.context_size,
            cost_usd = excluded.cost_usd, retired = excluded.retired",
        rusqlite::params![
            thread.id,
            thread.workspace_id,
            thread.title,
            thread.engine,
            thread.session_id,
            thread.status.as_str(),
            thread.used_tokens,
            thread.context_size,
            thread.cost_usd,
            thread.retired as i64,
            thread.parent_thread_id,
            thread.role.map(ThreadRole::as_str),
        ],
    )?;
    Ok(())
}

fn configure(conn: Connection, rust: &[RustMigration]) -> Result<Connection> {
    conn.pragma_update(None, "foreign_keys", true)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    tune(&conn)?;
    migrate::run(&conn, rust)?;
    Ok(conn)
}

/// Trades a power cut losing the newest commits (never corruption, under WAL) for no sync per
/// commit, and keeps pages and temp tables in memory.
fn tune(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "cache_size", -16_000)?;
    conn.pragma_update(None, "mmap_size", 256_i64 * 1024 * 1024)?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    conn.set_prepared_statement_cache_capacity(STATEMENT_CACHE);
    Ok(())
}

const REPO_COLUMNS: &str = "SELECT r.id, r.path, r.name, r.default_branch,
        COALESCE(c.setup_commands, ''), c.run_command, COALESCE(c.single_instance, 0) != 0
     FROM repos r LEFT JOIN repo_config c ON c.repo_id = r.id";

fn row_to_repo(row: &rusqlite::Row<'_>) -> rusqlite::Result<RepoRecord> {
    Ok(RepoRecord {
        id: row.get(0)?,
        path: row.get(1)?,
        name: row.get(2)?,
        default_branch: row
            .get::<_, Option<String>>(3)?
            .filter(|branch| !branch.trim().is_empty())
            .unwrap_or_else(|| crate::model::FALLBACK_DEFAULT_BRANCH.into()),
        setup_commands: row.get(4)?,
        run_command: row.get(5)?,
        single_instance: row.get(6)?,
    })
}

/// `WorkspaceRow`'s columns, in the order `workspace_at` reads them.
const WORKSPACE_FIELDS: [&str; 15] = [
    "id",
    "repo_id",
    "name",
    "branch",
    "base",
    "worktree_path",
    "status",
    "created_at",
    "summary",
    "summary_at",
    "kind",
    "pr_number",
    "pr_html_url",
    "archived_at",
    "run_command",
];

/// `ThreadRow`'s columns, in the order `thread_at` reads them.
const THREAD_FIELDS: [&str; 12] = [
    "id",
    "workspace_id",
    "title",
    "role",
    "engine",
    "session_id",
    "status",
    "used_tokens",
    "context_size",
    "cost_usd",
    "retired",
    "parent_thread_id",
];

fn qualified(alias: &str, fields: &[&str]) -> String {
    fields
        .iter()
        .map(|field| format!("{alias}.{field}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The select list `workspace_at` reads, from a workspace table aliased `alias`.
pub(crate) fn workspace_columns(alias: &str) -> String {
    qualified(alias, &WORKSPACE_FIELDS)
}

/// The select list `thread_at` reads, from a thread table aliased `alias`.
pub(crate) fn thread_columns(alias: &str) -> String {
    qualified(alias, &THREAD_FIELDS)
}

/// How many columns `thread_columns` selects, where a joined row's next table starts.
pub(crate) const THREAD_WIDTH: usize = THREAD_FIELDS.len();

fn select_workspaces(from: &str, rest: &str) -> String {
    format!("SELECT {} FROM {from} w {rest}", workspace_columns("w"))
}

fn select_threads(from: &str, rest: &str) -> String {
    format!("SELECT {} FROM {from} t {rest}", thread_columns("t"))
}

pub(crate) fn thread_at(row: &rusqlite::Row<'_>, at: usize) -> rusqlite::Result<ThreadRow> {
    Ok(ThreadRow {
        id: row.get(at)?,
        workspace_id: row.get(at + 1)?,
        title: row.get(at + 2)?,
        role: ThreadRole::parse(row.get::<_, Option<String>>(at + 3)?.as_deref()),
        engine: row.get(at + 4)?,
        session_id: row.get(at + 5)?,
        status: ThreadStatus::parse(&row.get::<_, String>(at + 6)?),
        used_tokens: row.get(at + 7)?,
        context_size: row.get(at + 8)?,
        cost_usd: row.get(at + 9)?,
        retired: row.get::<_, i64>(at + 10)? != 0,
        parent_thread_id: row.get(at + 11)?,
    })
}

fn row_to_thread(row: &rusqlite::Row<'_>) -> rusqlite::Result<ThreadRow> {
    thread_at(row, 0)
}

pub(crate) fn workspace_at(row: &rusqlite::Row<'_>, at: usize) -> rusqlite::Result<WorkspaceRow> {
    Ok(WorkspaceRow {
        id: row.get(at)?,
        repo_id: row.get(at + 1)?,
        name: row.get(at + 2)?,
        branch: row.get(at + 3)?,
        base: row.get(at + 4)?,
        worktree_path: row.get(at + 5)?,
        status: WorkspaceLifecycle::parse(&row.get::<_, String>(at + 6)?),
        created_at_ms: crate::time::sqlite_to_unix_ms(&row.get::<_, String>(at + 7)?),
        summary: row.get(at + 8)?,
        summary_at: row.get(at + 9)?,
        kind: WorkspaceKind::parse(row.get::<_, Option<String>>(at + 10)?.as_deref()),
        pr_number: row.get(at + 11)?,
        pr_html_url: row.get(at + 12)?,
        archived_at: row.get(at + 13)?,
        run_command: row.get(at + 14)?,
    })
}

fn row_to_workspace(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkspaceRow> {
    workspace_at(row, 0)
}

pub(crate) fn query_all<T>(
    conn: &Connection,
    sql: &str,
    f: impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>> {
    let mut stmt = conn.prepare_cached(sql)?;
    let rows = stmt.query_map([], f)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub(crate) fn settings_in(conn: &Connection) -> Result<Vec<(String, String)>> {
    query_all(conn, "SELECT key, value FROM settings", |row| {
        Ok((row.get(0)?, row.get(1)?))
    })
}

pub(crate) fn repos_in(conn: &Connection) -> Result<Vec<RepoRecord>> {
    query_all(conn, REPO_COLUMNS, row_to_repo)
}

pub(crate) fn live_workspaces_in(conn: &Connection) -> Result<Vec<WorkspaceRow>> {
    query_all(
        conn,
        &select_workspaces("live_workspaces", "ORDER BY w.position"),
        row_to_workspace,
    )
}

/// Scratches' threads aren't listed.
pub(crate) fn workspace_threads_in(conn: &Connection) -> Result<Vec<ThreadRow>> {
    query_all(
        conn,
        &select_threads(
            "visible_threads",
            "WHERE t.workspace_id IN (SELECT id FROM live_workspaces)
             ORDER BY t.created_at, t.position",
        ),
        row_to_thread,
    )
}

#[cfg(test)]
mod tests {
    use super::fixtures::{repo, thread};
    use super::*;

    #[test]
    fn reads_go_to_their_own_connection_and_see_committed_writes() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("cormux.db"), &[]).unwrap();
        store.set_setting("k", "v").unwrap();
        assert_eq!(store.get_setting("k").unwrap().as_deref(), Some("v"));
        let refused = store.with_read(|conn| Ok(conn.execute("DELETE FROM settings", [])?));
        assert!(refused.is_err(), "the reader is read-only");
        let synchronous: i64 = store
            .with_conn(|conn| Ok(conn.query_row("PRAGMA synchronous", [], |row| row.get(0))?))
            .unwrap();
        assert_eq!(synchronous, 1, "NORMAL");
    }

    #[test]
    fn idling_a_thread_settles_its_workspace_from_the_siblings() {
        use ThreadStatus::{Idle, Running};
        let store = Store::in_memory().unwrap();
        store.seed_workspace("w1", WorkspaceLifecycle::Running);
        store.seed_thread("a", "w1", Running);
        store.seed_thread("b", "w1", Running);
        let status = |store: &Store| store.workspace_by_id("w1").unwrap().unwrap().status;

        store.set_thread_status("a", Idle).unwrap();
        assert_eq!(status(&store), WorkspaceLifecycle::Running);
        store.set_thread_status("b", Idle).unwrap();
        assert_eq!(status(&store), WorkspaceLifecycle::Idle);
        store.set_thread_status("a", Running).unwrap();
        assert_eq!(status(&store), WorkspaceLifecycle::Running);

        store.archive_workspace("w1").unwrap();
        store.set_thread_status("a", Idle).unwrap();
        assert_eq!(status(&store), WorkspaceLifecycle::Running);
    }

    #[test]
    fn a_thread_moving_leaves_a_workspace_in_setup_or_teardown_alone() {
        let store = Store::in_memory().unwrap();
        store.seed_workspace("w1", WorkspaceLifecycle::Provisioning);
        store.seed_thread("a", "w1", ThreadStatus::Running);
        for held in [
            WorkspaceLifecycle::Provisioning,
            WorkspaceLifecycle::ProvisioningFailed,
            WorkspaceLifecycle::TearingDown,
        ] {
            store.set_workspace_status("w1", held).unwrap();
            store.set_thread_status("a", ThreadStatus::Idle).unwrap();
            store.set_thread_status("a", ThreadStatus::Running).unwrap();
            assert_eq!(store.workspace_by_id("w1").unwrap().unwrap().status, held);
        }
    }

    #[test]
    fn thread_session_and_model_survive_an_upsert() {
        let store = Store::in_memory().unwrap();
        store.seed_workspace("w1", WorkspaceLifecycle::Idle);
        store.seed_thread("lead", "w1", ThreadStatus::Running);
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

        assert_eq!(store.thread_effort("lead").unwrap(), None);
        store.set_thread_effort("lead", Some("high")).unwrap();
        assert_eq!(
            store.thread_effort("lead").unwrap().as_deref(),
            Some("high")
        );
        store.set_thread_effort("lead", None).unwrap();
        assert_eq!(store.thread_effort("lead").unwrap(), None);
        let lead = store.thread_by_id("lead").unwrap().unwrap();
        assert_eq!((lead.used_tokens, lead.context_size), (None, None));
    }

    #[test]
    fn a_restart_ends_the_turns_that_were_running() {
        let store = Store::in_memory().unwrap();
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

        let approval = |id: &str| store.get_approval(id).unwrap().unwrap().status;
        assert_eq!(approval("stale"), ApprovalStatus::Denied);
        assert_eq!(approval("done"), ApprovalStatus::Approved);
        let status = |id: &str| store.thread_by_id(id).unwrap().unwrap().status;
        assert_eq!(status("busy"), ThreadStatus::Idle);
        assert_eq!(status("setup"), ThreadStatus::Provisioning);
    }

    #[test]
    fn rows_read_back_as_written() {
        let store = Store::in_memory().unwrap();
        store
            .upsert_repo(&RepoRecord {
                setup_commands: "pnpm install".into(),
                run_command: Some("pnpm dev".into()),
                ..repo("r1")
            })
            .unwrap();
        store.seed_workspace("w1", WorkspaceLifecycle::Ready);
        store.seed_thread("t1", "w1", ThreadStatus::Idle);

        store
            .set_workspace_summary("w1", "Working on login.")
            .unwrap();
        let row = store.workspace_by_id("w1").unwrap().unwrap();
        assert_eq!(row.summary.as_deref(), Some("Working on login."));
        assert!(row.summary_at.is_some());
        assert_eq!(row.status, WorkspaceLifecycle::Ready);

        let repo = store.repo_by_id("r1").unwrap().unwrap();
        assert_eq!(repo.run_command.as_deref(), Some("pnpm dev"));
        assert!(store.repo_by_id("nope").unwrap().is_none());
        assert!(matches!(store.repo("nope"), Err(Error::Invalid(_))));
        assert_eq!(store.own_thread("w1").unwrap().unwrap().id, "t1");
        assert!(store.own_thread("w2").unwrap().is_none());
        assert_eq!(store.threads_for_workspace("w1").unwrap().len(), 1);
        assert!(store.threads_for_workspace("w2").unwrap().is_empty());
        assert_eq!(
            store.workspace_threads().unwrap(),
            [thread("t1", "w1", ThreadStatus::Idle)]
        );
    }

    /// TS used to resolve a blank default branch itself (to `main`, where the core kept `""`).
    #[test]
    fn a_repo_without_a_recorded_default_branch_reads_as_main() {
        let store = Store::in_memory().unwrap();
        store.upsert_repo(&super::fixtures::repo("r1")).unwrap();
        for stored in [None, Some(""), Some("  ")] {
            store
                .with_conn(|conn| {
                    conn.execute("UPDATE repos SET default_branch = ?1", [stored])?;
                    Ok(())
                })
                .unwrap();
            assert_eq!(store.repo("r1").unwrap().default_branch, "main");
        }
    }
}
