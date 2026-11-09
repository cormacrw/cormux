//! References: what a thread's agent linked to its work with `attach_entity` (a PR, a ticket, a
//! doc), kept in `thread_entities`.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::Result;
use crate::store::{Store, query_all};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EntityRow {
    pub id: i64,
    pub thread_id: String,
    pub name: String,
    pub url: String,
    /// SQLite `datetime('now')`, UTC, `YYYY-MM-DD HH:MM:SS`.
    pub created_at: String,
}

const ENTITY_FIELDS: &str = "id, thread_id, name, url, created_at";

pub(crate) fn visible(conn: &Connection) -> Result<Vec<EntityRow>> {
    query_all(
        conn,
        &format!(
            "SELECT {ENTITY_FIELDS} FROM thread_entities
             WHERE thread_id IN (SELECT id FROM visible_threads)
             ORDER BY id"
        ),
        row_to_entity,
    )
}

impl Store {
    /// Attaches `url` to the thread, or renames it if it's already attached.
    pub fn attach_entity(&self, thread_id: &str, name: &str, url: &str) -> Result<EntityRow> {
        self.with_conn(|conn| {
            Ok(conn.query_row(
                &format!(
                    "INSERT INTO thread_entities (thread_id, name, url) VALUES (?1, ?2, ?3)
                     ON CONFLICT(thread_id, url) DO UPDATE SET name = excluded.name
                     RETURNING {ENTITY_FIELDS}"
                ),
                rusqlite::params![thread_id, name, url],
                row_to_entity,
            )?)
        })
    }
}

fn row_to_entity(row: &rusqlite::Row<'_>) -> rusqlite::Result<EntityRow> {
    Ok(EntityRow {
        id: row.get(0)?,
        thread_id: row.get(1)?,
        name: row.get(2)?,
        url: row.get(3)?,
        created_at: row.get(4)?,
    })
}
