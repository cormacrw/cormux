//! `review_state`: where each workspace's review stands.

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::Result;
use crate::store::{Store, query_all};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ReviewStatus {
    /// The Reviewer is reading the changes.
    Running,
    /// The findings are in.
    Ready,
}

impl ReviewStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Ready => "ready",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "running" => Some(Self::Running),
            "ready" => Some(Self::Ready),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ReviewRow {
    pub workspace_id: String,
    /// None before a branch review starts.
    pub status: Option<ReviewStatus>,
    /// The review was posted to GitHub.
    pub submitted: bool,
}

const REVIEW_COLUMNS: &str = "SELECT workspace_id, status, submitted FROM review_state";

pub(crate) fn live(conn: &Connection) -> Result<Vec<ReviewRow>> {
    query_all(
        conn,
        &format!(
            "{REVIEW_COLUMNS} WHERE workspace_id IN (SELECT id FROM live_workspaces)
             ORDER BY rowid"
        ),
        row_to_review,
    )
}

impl Store {
    pub fn review(&self, workspace_id: &str) -> Result<Option<ReviewRow>> {
        self.with_read(|conn| {
            Ok(conn
                .prepare_cached(&format!("{REVIEW_COLUMNS} WHERE workspace_id = ?1"))?
                .query_row([workspace_id], row_to_review)
                .optional()?)
        })
    }

    pub fn set_review_status(&self, workspace_id: &str, status: ReviewStatus) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO review_state (workspace_id, status) VALUES (?1, ?2)
                 ON CONFLICT(workspace_id) DO UPDATE SET status = excluded.status",
                rusqlite::params![workspace_id, status.as_str()],
            )?;
            Ok(())
        })
    }

    pub fn finish_review(&self, workspace_id: &str, found: usize) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO review_state (workspace_id, status, has_findings) VALUES (?1, ?2, ?3)
                 ON CONFLICT(workspace_id) DO UPDATE
                    SET status = excluded.status, has_findings = excluded.has_findings",
                rusqlite::params![workspace_id, ReviewStatus::Ready.as_str(), found > 0],
            )?;
            Ok(())
        })
    }

    /// A review that stopped before it found anything: the workspace is back where its last
    /// finished review left it, or where it was before any.
    pub fn end_unfinished_review(&self, workspace_id: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE review_state SET status = CASE WHEN has_findings THEN ?2 ELSE NULL END
                 WHERE workspace_id = ?1 AND status = ?3",
                rusqlite::params![
                    workspace_id,
                    ReviewStatus::Ready.as_str(),
                    ReviewStatus::Running.as_str()
                ],
            )?;
            Ok(())
        })
    }

    pub fn mark_review_submitted(&self, workspace_id: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO review_state (workspace_id, submitted) VALUES (?1, 1)
                 ON CONFLICT(workspace_id) DO UPDATE SET submitted = 1",
                [workspace_id],
            )?;
            Ok(())
        })
    }
}

fn row_to_review(row: &rusqlite::Row<'_>) -> rusqlite::Result<ReviewRow> {
    Ok(ReviewRow {
        workspace_id: row.get(0)?,
        status: row
            .get::<_, Option<String>>(1)?
            .as_deref()
            .and_then(ReviewStatus::parse),
        submitted: row.get::<_, i64>(2)? != 0,
    })
}
