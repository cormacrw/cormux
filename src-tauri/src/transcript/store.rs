//! The `thread_events` table, which only the transcript writes and reads.

use std::sync::LazyLock;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::Result;
use crate::store::{Store, query_all};

use super::{EXIT, MESSAGE, TOOL, TURN_END};

const THREAD_EVENT_PAGE: u32 = 2_000;
const THREAD_EVENT_PAGE_MAX: u32 = 10_000;

#[derive(Debug, Clone)]
pub struct ThreadEventRow {
    pub seq: i64,
    pub kind: String,
    pub payload: String,
    /// When it was saved, in Unix milliseconds (0 if the stored time can't be read).
    pub at_ms: i64,
}

/// A window of a thread's rows, oldest first.
#[derive(Debug, Clone)]
pub struct ThreadEventPage {
    pub events: Vec<ThreadEventRow>,
    pub has_older: bool,
    pub has_newer: bool,
}

/// `created_at` (SQLite `datetime('now')`, UTC to the second) as Unix milliseconds.
const AT_MS: &str = "COALESCE(CAST(strftime('%s', created_at) AS INTEGER) * 1000, 0)";

/// The last message from the user or end of a turn. Streamed output doesn't count, so a working
/// thread holds its place in the sidebar.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ThreadActivityRow {
    pub thread_id: String,
    /// Unix milliseconds.
    pub at_ms: i64,
}

/// What the row-rewriting migration does with one transcript row.
pub enum Rewrite {
    Keep,
    Replace { kind: String, payload: String },
    Delete,
}

/// Rewrites every transcript row, thread by thread in order, and every approval payload.
pub(super) fn rewrite_rows(
    conn: &Connection,
    mut event: impl FnMut(&str, i64, &str, &str) -> Rewrite,
    mut approval: impl FnMut(&str) -> Option<String>,
) -> Result<()> {
    let rows = {
        let mut stmt = conn.prepare(
            "SELECT id, thread_id, seq, kind, payload FROM thread_events
             ORDER BY thread_id, seq",
        )?;
        stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (id, thread_id, seq, kind, payload) in rows {
        match event(&thread_id, seq, &kind, &payload) {
            Rewrite::Keep => {}
            Rewrite::Replace { kind, payload } => {
                conn.execute(
                    "UPDATE thread_events SET kind = ?1, payload = ?2 WHERE id = ?3",
                    rusqlite::params![kind, payload, id],
                )?;
            }
            Rewrite::Delete => {
                conn.execute("DELETE FROM thread_events WHERE id = ?1", [id])?;
            }
        }
    }
    let approvals = {
        let mut stmt = conn.prepare("SELECT id, payload FROM approvals")?;
        stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (id, payload) in approvals {
        if let Some(payload) = approval(&payload) {
            conn.execute(
                "UPDATE approvals SET payload = ?1 WHERE id = ?2",
                rusqlite::params![payload, id],
            )?;
        }
    }
    Ok(())
}

/// When each visible thread last moved: its newest turn end, engine exit or user message.
/// Threads that never have are left out. The row condition must stay word for word what the
/// `thread_events_activity` partial index (migration 020) declares, or SQLite walks every
/// streamed chunk of a thread mid-turn.
static ACTIVITY_SQL: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT thread_id,
        COALESCE(CAST(strftime('%s', created_at) AS INTEGER) * 1000, 0) FROM (
        SELECT t.id AS thread_id, (
            SELECT e.created_at FROM thread_events e
            WHERE e.thread_id = t.id
              AND (e.kind IN ('{TURN_END}', '{EXIT}')
                   OR (e.kind = '{MESSAGE}' AND json_extract(e.payload, '$.role') = 'user'))
            ORDER BY e.seq DESC LIMIT 1
        ) AS created_at
        FROM visible_threads t
     ) WHERE created_at IS NOT NULL"
    )
});

pub(crate) fn activity_in(conn: &Connection) -> Result<Vec<ThreadActivityRow>> {
    query_all(conn, &ACTIVITY_SQL, |row| {
        Ok(ThreadActivityRow {
            thread_id: row.get(0)?,
            at_ms: row.get(1)?,
        })
    })
}

impl Store {
    /// How SQLite runs the activity lookup, for checking it uses its index.
    #[cfg(test)]
    pub fn activity_query_plan(&self) -> Result<String> {
        self.with_read(|conn| {
            let mut stmt = conn.prepare(&format!("EXPLAIN QUERY PLAN {}", *ACTIVITY_SQL))?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(3))?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?.join("\n"))
        })
    }

    pub fn thread_event_page(
        &self,
        thread_id: &str,
        after_seq: Option<i64>,
        before_seq: Option<i64>,
        limit: Option<u32>,
    ) -> Result<ThreadEventPage> {
        let limit = limit
            .unwrap_or(THREAD_EVENT_PAGE)
            .clamp(1, THREAD_EVENT_PAGE_MAX) as i64;
        self.with_read(|conn| {
            let mut events = if let Some(after) = after_seq {
                let mut stmt = conn.prepare_cached(&format!(
                    "SELECT seq, kind, payload, {AT_MS} FROM thread_events
                     WHERE thread_id = ?1 AND seq > ?2 ORDER BY seq LIMIT ?3"
                ))?;
                stmt.query_map(rusqlite::params![thread_id, after, limit], row_to_event)?
                    .collect::<std::result::Result<Vec<_>, _>>()?
            } else {
                let mut stmt = conn.prepare_cached(&format!(
                    "SELECT seq, kind, payload, {AT_MS} FROM thread_events
                     WHERE thread_id = ?1 AND seq < ?2 ORDER BY seq DESC LIMIT ?3"
                ))?;
                let before = before_seq.unwrap_or(i64::MAX);
                stmt.query_map(rusqlite::params![thread_id, before, limit], row_to_event)?
                    .collect::<std::result::Result<Vec<_>, _>>()?
            };
            events.sort_by_key(|row| row.seq);
            let exists = |condition: &str, seq: i64| -> Result<bool> {
                Ok(conn
                    .prepare_cached(&format!(
                        "SELECT EXISTS (
                            SELECT 1 FROM thread_events WHERE thread_id = ?1 AND seq {condition} ?2
                         )"
                    ))?
                    .query_row(rusqlite::params![thread_id, seq], |row| row.get(0))?)
            };
            let has_older = match events.first() {
                Some(first) => exists("<", first.seq)?,
                None => false,
            };
            let has_newer = match events.last() {
                Some(last) => exists(">", last.seq)?,
                None => false,
            };
            Ok(ThreadEventPage {
                events,
                has_older,
                has_newer,
            })
        })
    }

    /// The thread's session steps as `(seq, payload)`, oldest first.
    pub fn session_steps(&self, thread_id: &str) -> Result<Vec<(i64, String)>> {
        let kinds = super::CONVERSATION_STEP_KINDS
            .map(|kind| format!("'{kind}'"))
            .join(", ");
        self.with_read(|conn| {
            let mut stmt = conn.prepare_cached(&format!(
                "SELECT seq, payload FROM thread_events
                 WHERE thread_id = ?1 AND kind = '{TOOL}'
                   AND CASE WHEN json_valid(payload) THEN json_extract(payload, '$.step.kind') END
                       IN ({kinds})
                 ORDER BY seq"
            ))?;
            let rows = stmt.query_map([thread_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }

    /// Every row of the thread as `(seq, at_ms, payload if it is a message)`, oldest first.
    pub fn message_rows(&self, thread_id: &str) -> Result<Vec<(i64, i64, Option<String>)>> {
        self.with_read(|conn| {
            let mut stmt = conn.prepare_cached(&format!(
                "SELECT seq, {AT_MS}, CASE WHEN kind = '{MESSAGE}' THEN payload END
                 FROM thread_events WHERE thread_id = ?1 ORDER BY seq"
            ))?;
            let rows = stmt.query_map([thread_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }

    /// The newest `seq` among the thread's rows of `kind`, or 0 if it has none.
    pub fn last_seq_of_kind(&self, thread_id: &str, kind: &str) -> Result<i64> {
        self.with_read(|conn| {
            Ok(conn.query_row(
                "SELECT COALESCE(MAX(seq), 0) FROM thread_events
                 WHERE thread_id = ?1 AND kind = ?2",
                rusqlite::params![thread_id, kind],
                |row| row.get(0),
            )?)
        })
    }

    /// The newest `seq` of the thread's steps of `step`'s kind, if it has one.
    pub fn last_step_seq(
        &self,
        thread_id: &str,
        step: &crate::timeline::TimelineStep,
    ) -> Result<Option<i64>> {
        let kind = serde_json::to_value(step)
            .map_err(|error| crate::error::Error::Store(error.to_string()))?["kind"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        self.with_read(|conn| {
            Ok(conn
                .prepare_cached(&format!(
                    "SELECT MAX(seq) FROM thread_events
                     WHERE thread_id = ?1 AND kind = '{TOOL}'
                       AND CASE WHEN json_valid(payload) THEN json_extract(payload, '$.step.kind') END = ?2"
                ))?
                .query_row(rusqlite::params![thread_id, kind], |row| row.get(0))?)
        })
    }

    /// Hands the thread's rows (only those of `kind`, if given) to `visit` as `(seq, kind,
    /// payload)`, oldest or newest first, until it returns false.
    pub fn visit_events(
        &self,
        thread_id: &str,
        kind: Option<&str>,
        newest_first: bool,
        mut visit: impl FnMut(i64, &str, &str) -> bool,
    ) -> Result<()> {
        let order = if newest_first { "DESC" } else { "ASC" };
        self.with_read(|conn| {
            let mut stmt = conn.prepare_cached(&format!(
                "SELECT seq, kind, payload FROM thread_events
                 WHERE thread_id = ?1 AND (?2 IS NULL OR kind = ?2) ORDER BY seq {order}"
            ))?;
            let mut rows = stmt.query(rusqlite::params![thread_id, kind])?;
            while let Some(row) = rows.next()? {
                let seq: i64 = row.get(0)?;
                let kind: String = row.get(1)?;
                let payload: String = row.get(2)?;
                if !visit(seq, &kind, &payload) {
                    break;
                }
            }
            Ok(())
        })
    }

    /// A page of the transcript after `since`, oldest first, as `(seq, kind, payload)`.
    pub fn thread_events_after(
        &self,
        thread_id: &str,
        since: i64,
        limit: i64,
    ) -> Result<Vec<(i64, String, String)>> {
        self.with_read(|conn| {
            let mut stmt = conn.prepare_cached(
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

    /// The hot path for every streamed chunk, so it's a single statement: the next `seq` is
    /// picked and the row written in one go, which also leaves no window for two appenders to
    /// pick the same number.
    pub fn append_event(&self, thread_id: &str, kind: &str, payload: &str) -> Result<i64> {
        self.with_conn(|conn| {
            let seq: i64 = conn
                .prepare_cached(
                    "INSERT INTO thread_events (thread_id, seq, kind, payload)
                     SELECT ?1, COALESCE(MAX(seq), 0) + 1, ?2, ?3
                     FROM thread_events WHERE thread_id = ?1
                     RETURNING seq",
                )?
                .query_row(rusqlite::params![thread_id, kind, payload], |row| {
                    row.get(0)
                })?;
            Ok(seq)
        })
    }
}

fn row_to_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<ThreadEventRow> {
    Ok(ThreadEventRow {
        seq: row.get(0)?,
        kind: row.get(1)?,
        payload: row.get(2)?,
        at_ms: row.get(3)?,
    })
}

#[cfg(test)]
mod tests {
    use crate::model::{ThreadStatus, WorkspaceLifecycle};
    use crate::store::Store;

    use super::ThreadEventPage;

    #[test]
    fn transcripts_page_from_the_newest_rows() {
        let store = Store::in_memory().unwrap();
        store.seed_workspace("w1", WorkspaceLifecycle::Idle);
        store.seed_thread("t1", "w1", ThreadStatus::Idle);
        for index in 0..5 {
            store
                .append_event("t1", "tool", &format!("{{\"n\":{index}}}"))
                .unwrap();
        }
        let seqs =
            |page: &ThreadEventPage| page.events.iter().map(|row| row.seq).collect::<Vec<_>>();

        let newest = store.thread_event_page("t1", None, None, Some(2)).unwrap();
        assert_eq!(seqs(&newest), vec![4, 5]);
        assert!(newest.has_older);

        let older = store
            .thread_event_page("t1", None, Some(4), Some(10))
            .unwrap();
        assert_eq!(seqs(&older), vec![1, 2, 3]);
        assert!(!older.has_older);

        assert!(older.has_newer);

        let catch_up = store.thread_event_page("t1", Some(3), None, None).unwrap();
        assert_eq!(seqs(&catch_up), vec![4, 5]);
        assert!(!catch_up.has_newer);
        assert!(
            catch_up.events[0].at_ms > 1_700_000_000_000,
            "stored time in ms"
        );

        let empty = store.thread_event_page("gone", None, None, None).unwrap();
        assert!(empty.events.is_empty() && !empty.has_older);
    }

    #[test]
    fn appended_events_number_from_one_per_thread() {
        let store = Store::in_memory().unwrap();
        store.seed_workspace("w1", WorkspaceLifecycle::Idle);
        for id in ["a", "b"] {
            store.seed_thread(id, "w1", ThreadStatus::Idle);
        }
        assert_eq!(store.append_event("a", "tool", "{}").unwrap(), 1);
        assert_eq!(store.append_event("a", "tool", "{}").unwrap(), 2);
        assert_eq!(store.append_event("b", "tool", "{}").unwrap(), 1);
        assert_eq!(store.append_event("a", "tool", "{}").unwrap(), 3);
        assert_eq!(store.last_seq_of_kind("a", "message").unwrap(), 0);
        assert_eq!(store.last_seq_of_kind("a", "tool").unwrap(), 3);
    }
}
