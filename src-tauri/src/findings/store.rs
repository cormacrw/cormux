//! The `findings` table.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{Error, Result};
use crate::store::{Store, query_all};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FindingRow {
    pub id: String,
    pub workspace_id: String,
    pub severity: String,
    pub title: String,
    pub file: Option<String>,
    pub line: Option<i64>,
    pub explanation: String,
    pub status: String,
    pub commit_sha: Option<String>,
    pub sent_to_thread_id: Option<String>,
}

const FINDING_COLUMNS: &str = "SELECT id, workspace_id, severity, title, file, line, explanation,
        status, commit_sha, sent_to_thread_id FROM findings";

/// The live workspaces' findings, oldest first.
pub(crate) fn live(conn: &Connection) -> Result<Vec<FindingRow>> {
    query_all(
        conn,
        &format!(
            "{FINDING_COLUMNS} WHERE workspace_id IN (SELECT id FROM live_workspaces)
             ORDER BY rowid"
        ),
        row_to_finding,
    )
}

impl Store {
    pub fn mark_finding_fixed(&self, finding_id: &str, commit_sha: &str) -> Result<()> {
        self.with_conn(|conn| {
            let changed = conn.execute(
                "UPDATE findings SET status = 'fixed', commit_sha = ?1 WHERE id = ?2",
                rusqlite::params![commit_sha, finding_id],
            )?;
            if changed == 0 {
                return Err(Error::Invalid(format!("unknown finding {finding_id}")));
            }
            Ok(())
        })
    }

    /// Marks every finding sent, or none of them: a finding that isn't open fails the batch.
    pub fn mark_findings_sent(&self, finding_ids: &[String], thread_id: &str) -> Result<()> {
        self.with_tx(|conn| {
            let mut stmt = conn.prepare_cached(
                "UPDATE findings SET status = 'sent', sent_to_thread_id = ?1
                 WHERE id = ?2 AND status = 'open'",
            )?;
            for id in finding_ids {
                if stmt.execute(rusqlite::params![thread_id, id])? == 0 {
                    return Err(Error::Invalid(format!(
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
        self.with_tx(|conn| {
            let mut stmt = conn.prepare_cached(
                "UPDATE findings SET status = ?1
                 WHERE id = ?2 AND workspace_id = ?3 AND status = ?4",
            )?;
            let mut changed = 0;
            for id in finding_ids {
                changed += stmt.execute(rusqlite::params![to, id, workspace_id, from])?;
            }
            Ok(changed)
        })
    }

    pub fn finding_by_id(&self, finding_id: &str) -> Result<Option<FindingRow>> {
        self.with_read(|conn| {
            let mut stmt = conn.prepare_cached(&format!("{FINDING_COLUMNS} WHERE id = ?1"))?;
            let mut rows = stmt.query([finding_id])?;
            match rows.next()? {
                Some(row) => Ok(Some(row_to_finding(row)?)),
                None => Ok(None),
            }
        })
    }

    /// The workspace's findings in the order they were reported.
    pub fn findings_for_workspace(&self, workspace_id: &str) -> Result<Vec<FindingRow>> {
        self.with_read(|conn| {
            let mut stmt = conn.prepare_cached(&format!(
                "{FINDING_COLUMNS} WHERE workspace_id = ?1 ORDER BY rowid"
            ))?;
            let rows = stmt.query_map([workspace_id], row_to_finding)?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }

    /// Swaps a workspace's findings for `findings` in one transaction, so a failed insert
    /// leaves the old set in place and a reader never sees it half replaced.
    pub fn replace_findings(&self, workspace_id: &str, findings: &[FindingRow]) -> Result<()> {
        self.with_tx(|conn| {
            conn.execute(
                "DELETE FROM findings WHERE workspace_id = ?1",
                [workspace_id],
            )?;
            for finding in findings {
                write_finding(conn, finding)?;
            }
            Ok(())
        })
    }

    #[cfg(test)]
    pub fn upsert_finding(&self, finding: &FindingRow) -> Result<()> {
        self.with_conn(|conn| write_finding(conn, finding))
    }
}

fn write_finding(conn: &Connection, finding: &FindingRow) -> Result<()> {
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
}

fn row_to_finding(row: &rusqlite::Row<'_>) -> rusqlite::Result<FindingRow> {
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
}
