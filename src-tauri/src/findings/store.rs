use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{Error, Result};
use crate::store::{Store, query_all};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FindingStatus {
    Open,
    /// Sent to a thread to fix.
    Sent,
    Fixed,
    /// Kept in the list, but not sent or posted.
    Dismissed,
}

impl FindingStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Sent => "sent",
            Self::Fixed => "fixed",
            Self::Dismissed => "dismissed",
        }
    }

    /// A stored status. Anything unrecognised reads as `Open`, so it can still be acted on.
    pub fn parse(raw: &str) -> Self {
        match raw {
            "sent" => Self::Sent,
            "fixed" => Self::Fixed,
            "dismissed" => Self::Dismissed,
            _ => Self::Open,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FindingSeverity {
    /// Must be fixed before merging.
    Blocking,
    /// Worth doing in this change.
    Suggestion,
    /// Optional polish.
    Nit,
}

impl FindingSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blocking => "blocking",
            Self::Suggestion => "suggestion",
            Self::Nit => "nit",
        }
    }

    /// What a Reviewer wrote, or a stored value. Anything unrecognised is a suggestion.
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "blocking" | "blocker" | "critical" | "high" => Self::Blocking,
            "nit" | "nitpick" | "low" => Self::Nit,
            _ => Self::Suggestion,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FindingRow {
    pub id: String,
    pub workspace_id: String,
    pub severity: FindingSeverity,
    pub title: String,
    pub file: Option<String>,
    pub line: Option<i64>,
    pub explanation: String,
    pub status: FindingStatus,
    pub commit_sha: Option<String>,
    pub sent_to_thread_id: Option<String>,
}

/// The findings a thread was last sent to fix, and the seq of the message that sent them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindingsBatch {
    pub ids: Vec<String>,
    pub message_seq: i64,
}

const FINDING_COLUMNS: &str = "SELECT id, workspace_id, severity, title, file, line, explanation,
        status, commit_sha, sent_to_thread_id FROM findings";

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
                "UPDATE findings SET status = ?1, commit_sha = ?2 WHERE id = ?3",
                rusqlite::params![FindingStatus::Fixed.as_str(), commit_sha, finding_id],
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
                "UPDATE findings SET status = ?1, sent_to_thread_id = ?2
                 WHERE id = ?3 AND status = ?4",
            )?;
            for id in finding_ids {
                let sent = FindingStatus::Sent.as_str();
                let open = FindingStatus::Open.as_str();
                if stmt.execute(rusqlite::params![sent, thread_id, id, open])? == 0 {
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
            (FindingStatus::Open, FindingStatus::Dismissed)
        } else {
            (FindingStatus::Dismissed, FindingStatus::Open)
        };
        self.with_tx(|conn| {
            let mut stmt = conn.prepare_cached(
                "UPDATE findings SET status = ?1
                 WHERE id = ?2 AND workspace_id = ?3 AND status = ?4",
            )?;
            let mut changed = 0;
            for id in finding_ids {
                changed += stmt.execute(rusqlite::params![
                    to.as_str(),
                    id,
                    workspace_id,
                    from.as_str()
                ])?;
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
    pub(super) fn replace_findings(
        &self,
        workspace_id: &str,
        findings: &[FindingRow],
    ) -> Result<()> {
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

    /// The findings `thread_id` was last sent to fix, which it works through as one batch; none
    /// once it fixed them or never got them. A batch that can't be read is logged and reads as none.
    pub fn findings_batch(&self, thread_id: &str) -> Result<Option<FindingsBatch>> {
        let raw: Option<String> = self.with_read(|conn| {
            Ok(conn
                .query_row(
                    "SELECT findings_batch FROM threads WHERE id = ?1",
                    [thread_id],
                    |row| row.get(0),
                )
                .optional()?
                .flatten())
        })?;
        Ok(raw.and_then(|raw| {
            serde_json::from_str(&raw)
                .inspect_err(|error| log::warn!("reading {thread_id}'s findings batch: {error}"))
                .ok()
        }))
    }

    pub fn set_findings_batch(&self, thread_id: &str, batch: Option<&FindingsBatch>) -> Result<()> {
        let batch = batch
            .map(serde_json::to_string)
            .transpose()
            .map_err(|error| Error::Store(error.to_string()))?;
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE threads SET findings_batch = ?1 WHERE id = ?2",
                rusqlite::params![batch, thread_id],
            )?;
            Ok(())
        })
    }

    /// Findings sent to `thread_id` that it never got go back to open, to be sent again.
    pub fn reopen_findings(&self, finding_ids: &[String], thread_id: &str) -> Result<()> {
        self.with_tx(|conn| {
            let mut stmt = conn.prepare_cached(
                "UPDATE findings SET status = ?1, sent_to_thread_id = NULL
                 WHERE id = ?2 AND status = ?3 AND sent_to_thread_id = ?4",
            )?;
            for id in finding_ids {
                stmt.execute(rusqlite::params![
                    FindingStatus::Open.as_str(),
                    id,
                    FindingStatus::Sent.as_str(),
                    thread_id
                ])?;
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
            finding.severity.as_str(),
            finding.title,
            finding.file,
            finding.line,
            finding.explanation,
            finding.status.as_str(),
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
        severity: FindingSeverity::parse(&row.get::<_, String>(2)?),
        title: row.get(3)?,
        file: row.get(4)?,
        line: row.get(5)?,
        explanation: row.get(6)?,
        status: FindingStatus::parse(&row.get::<_, String>(7)?),
        commit_sha: row.get(8)?,
        sent_to_thread_id: row.get(9)?,
    })
}
