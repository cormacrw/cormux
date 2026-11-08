//! The `approvals` table: one row per card, its status and what it shows.

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{Error, Result};
use crate::model::{ApprovalPayload, ApprovalStatus};
use crate::store::{Store, query_all};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRow {
    pub id: String,
    pub thread_id: String,
    pub status: ApprovalStatus,
    pub tool: String,
    /// None when the stored card can't be read; the timeline falls back to the permission event.
    pub payload: Option<ApprovalPayload>,
}

const APPROVAL_COLUMNS: &str = "SELECT id, thread_id, status, tool, payload FROM approvals";

/// The visible threads' approvals, oldest first. Closed threads' answers stay in the table for
/// the record but never reach the webview.
pub(crate) fn visible(conn: &Connection) -> Result<Vec<ApprovalRow>> {
    query_all(
        conn,
        &format!(
            "{APPROVAL_COLUMNS} WHERE thread_id IN (SELECT id FROM visible_threads)
             ORDER BY rowid"
        ),
        row_to_approval,
    )
}

impl Store {
    pub fn get_approval(&self, id: &str) -> Result<Option<ApprovalRow>> {
        self.with_read(|conn| {
            Ok(conn
                .prepare_cached(&format!("{APPROVAL_COLUMNS} WHERE id = ?1"))?
                .query_row([id], row_to_approval)
                .optional()?)
        })
    }

    pub fn pending_approvals_for_thread(&self, thread_id: &str) -> Result<Vec<ApprovalRow>> {
        self.with_read(|conn| {
            let mut stmt = conn.prepare_cached(&format!(
                "{APPROVAL_COLUMNS} WHERE thread_id = ?1 AND status = ?2 ORDER BY rowid"
            ))?;
            let rows = stmt.query_map(
                rusqlite::params![thread_id, ApprovalStatus::Pending.as_str()],
                row_to_approval,
            )?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }

    pub fn pending_approval_count(&self) -> Result<usize> {
        self.with_read(|conn| {
            let count: i64 = conn
                .prepare_cached("SELECT COUNT(*) FROM approvals WHERE status = ?1")?
                .query_row([ApprovalStatus::Pending.as_str()], |row| row.get(0))?;
            Ok(count as usize)
        })
    }

    pub fn insert_pending_approval(
        &self,
        id: &str,
        thread_id: &str,
        tool: &str,
        payload: &ApprovalPayload,
    ) -> Result<()> {
        let payload = encode(payload)?;
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO approvals (id, thread_id, status, tool, payload)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET status = excluded.status, payload = excluded.payload",
                rusqlite::params![
                    id,
                    thread_id,
                    ApprovalStatus::Pending.as_str(),
                    tool,
                    payload
                ],
            )?;
            Ok(())
        })
    }

    /// Overwrites an approval's stored card with raw text, the way an older or damaged row reads.
    #[cfg(test)]
    pub fn set_approval_payload_text(&self, id: &str, text: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE approvals SET payload = ?1 WHERE id = ?2",
                rusqlite::params![text, id],
            )?;
            Ok(())
        })
    }

    #[cfg(test)]
    pub fn approval_payload_text(&self, id: &str) -> Result<Option<String>> {
        self.with_read(|conn| {
            Ok(conn
                .query_row("SELECT payload FROM approvals WHERE id = ?1", [id], |row| {
                    row.get(0)
                })
                .optional()?)
        })
    }

    /// Decides a pending approval. `payload` replaces the stored card; none leaves it as it is.
    /// False if the approval wasn't pending.
    pub fn decide_approval(
        &self,
        id: &str,
        status: ApprovalStatus,
        payload: Option<&ApprovalPayload>,
    ) -> Result<bool> {
        let payload = payload.map(encode).transpose()?;
        self.with_conn(|conn| {
            let changed = conn.execute(
                "UPDATE approvals SET status = ?1, payload = COALESCE(?2, payload)
                 WHERE id = ?3 AND status = ?4",
                rusqlite::params![
                    status.as_str(),
                    payload,
                    id,
                    ApprovalStatus::Pending.as_str()
                ],
            )?;
            Ok(changed > 0)
        })
    }

    /// Approvals nothing can answer any more: the turn that asked has ended. Left pending,
    /// they would hold the thread in "needs attention" for good.
    pub fn deny_pending_approvals(&self, thread_id: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE approvals SET status = ?1 WHERE status = ?2 AND thread_id = ?3",
                rusqlite::params![
                    ApprovalStatus::Denied.as_str(),
                    ApprovalStatus::Pending.as_str(),
                    thread_id
                ],
            )?;
            Ok(())
        })
    }
}

fn encode(payload: &ApprovalPayload) -> Result<String> {
    serde_json::to_string(payload).map_err(|error| Error::Store(error.to_string()))
}

fn row_to_approval(row: &rusqlite::Row<'_>) -> rusqlite::Result<ApprovalRow> {
    let payload: String = row.get(4)?;
    Ok(ApprovalRow {
        id: row.get(0)?,
        thread_id: row.get(1)?,
        status: ApprovalStatus::parse(&row.get::<_, String>(2)?),
        tool: row.get(3)?,
        payload: serde_json::from_str(&payload).ok(),
    })
}
