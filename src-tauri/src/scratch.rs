//! Scratches: one-off conversations from before every thread had its own worktree. They can't
//! be started or continued any more; what's left stays readable until it's deleted.

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{Error, Result};
use crate::feedback::Change;
use crate::model::ThreadStatus;
use crate::state::AppState;
use crate::store::{Store, query_all};

/// A titled one-off conversation against a repo checkout. Its thread's owner id is the scratch id.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScratchRow {
    pub id: String,
    pub repo_id: String,
    pub title: String,
    pub thread_id: String,
    pub engine: String,
    pub status: ThreadStatus,
    /// SQLite `datetime('now')`, UTC, `YYYY-MM-DD HH:MM:SS`.
    pub created_at: String,
}

const SCRATCH_COLUMNS: &str = "SELECT s.id, s.repo_id, s.title, s.thread_id, t.engine, t.status,
        s.created_at
     FROM scratches s JOIN threads t ON t.id = s.thread_id";

/// Every scratch, newest first.
pub(crate) fn all(conn: &Connection) -> Result<Vec<ScratchRow>> {
    query_all(
        conn,
        &format!("{SCRATCH_COLUMNS} ORDER BY s.created_at DESC, s.rowid DESC"),
        row_to_scratch,
    )
}

impl Store {
    #[cfg(test)]
    pub fn insert_scratch(
        &self,
        id: &str,
        repo_id: &str,
        title: &str,
        thread_id: &str,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO scratches (id, repo_id, title, thread_id) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![id, repo_id, title, thread_id],
            )?;
            Ok(())
        })
    }

    pub fn scratch_by_id(&self, scratch_id: &str) -> Result<Option<ScratchRow>> {
        self.with_read(|conn| {
            Ok(conn
                .prepare_cached(&format!("{SCRATCH_COLUMNS} WHERE s.id = ?1"))?
                .query_row([scratch_id], row_to_scratch)
                .optional()?)
        })
    }

    /// Removes the scratch and its thread; events and approvals cascade with the thread.
    pub fn delete_scratch(&self, scratch_id: &str) -> Result<()> {
        self.with_tx(|conn| {
            let thread_id: Option<String> = conn
                .query_row(
                    "SELECT thread_id FROM scratches WHERE id = ?1",
                    [scratch_id],
                    |row| row.get(0),
                )
                .optional()?;
            conn.execute("DELETE FROM scratches WHERE id = ?1", [scratch_id])?;
            if let Some(thread_id) = thread_id {
                conn.execute("DELETE FROM threads WHERE id = ?1", [thread_id])?;
            }
            Ok(())
        })
    }
}

fn row_to_scratch(row: &rusqlite::Row<'_>) -> rusqlite::Result<ScratchRow> {
    Ok(ScratchRow {
        id: row.get(0)?,
        repo_id: row.get(1)?,
        title: row.get(2)?,
        thread_id: row.get(3)?,
        engine: row.get(4)?,
        status: ThreadStatus::parse(&row.get::<_, String>(5)?),
        created_at: row.get(6)?,
    })
}

/// Discards the scratch's conversation. The repo is not changed.
#[tauri::command]
#[specta::specta]
pub async fn end_scratch(scratch_id: String, state: tauri::State<'_, AppState>) -> Result<()> {
    if state.store.scratch_by_id(&scratch_id)?.is_none() {
        return Err(Error::Invalid(format!("unknown scratch {scratch_id}")));
    }
    state.store.delete_scratch(&scratch_id)?;
    state.out.changed(Change::THREADS);
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::model::ThreadStatus;
    use crate::store::Store;
    use crate::store::fixtures::thread;
    use crate::store::types::ThreadRow;

    #[test]
    fn scratch_keeps_its_thread_out_of_workspace_lists() {
        let store = Store::in_memory().unwrap();
        store
            .upsert_thread(&ThreadRow {
                role: None,
                ..thread("t1", "scratch-1", ThreadStatus::Running)
            })
            .unwrap();
        store
            .insert_scratch("scratch-1", "gone-repo", "Why", "t1")
            .unwrap();
        store.append_event("t1", "message", "{}").unwrap();
        store.set_thread_status("t1", ThreadStatus::Idle).unwrap();

        let snapshot = crate::snapshot::read(&store);
        assert!(snapshot.threads.is_empty());
        assert_eq!(snapshot.scratches.len(), 1);
        assert_eq!(snapshot.scratches[0].engine, "claude");
        assert_eq!(snapshot.scratches[0].status, ThreadStatus::Idle);
        assert!(store.thread_by_id("t1").unwrap().is_some());

        store.delete_scratch("scratch-1").unwrap();
        let snapshot = crate::snapshot::read(&store);
        assert!(snapshot.scratches.is_empty());
        assert!(snapshot.thread_activity.is_empty());
        assert!(store.thread_by_id("t1").unwrap().is_none());
    }
}
