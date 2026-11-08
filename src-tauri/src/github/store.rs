//! `pr_cache`: the open pull requests from the last sync, and the ones Cormux opened since.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{Error, Result};
use crate::store::{Store, query_all};

use super::types::PullRequestPayload;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PrRow {
    pub id: String,
    pub repo_id: Option<String>,
    pub number: i64,
    pub title: String,
    pub payload: PullRequestPayload,
}

impl From<&PullRequestPayload> for PrRow {
    fn from(pr: &PullRequestPayload) -> Self {
        Self {
            id: pr.cache_id(),
            repo_id: pr.repo_id.clone(),
            number: pr.num,
            title: pr.title.clone(),
            payload: pr.clone(),
        }
    }
}

const PR_COLUMNS: &str = "SELECT id, repo_id, number, title, payload FROM pr_cache";

/// A row whose payload no longer reads is left out.
pub(crate) fn all(conn: &Connection) -> Result<Vec<PrRow>> {
    let rows = query_all(conn, PR_COLUMNS, |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;
    Ok(rows
        .into_iter()
        .filter_map(
            |(id, repo_id, number, title, payload)| match serde_json::from_str(&payload) {
                Ok(payload) => Some(PrRow {
                    id,
                    repo_id,
                    number,
                    title,
                    payload,
                }),
                Err(error) => {
                    log::warn!("cached pull request {id} doesn't read: {error}");
                    None
                }
            },
        )
        .collect())
}

impl Store {
    pub fn cached_pull_request(
        &self,
        repo_id: &str,
        number: i64,
    ) -> Result<Option<PullRequestPayload>> {
        Ok(self
            .with_read(all)?
            .into_iter()
            .find(|row| row.repo_id.as_deref() == Some(repo_id) && row.number == number)
            .map(|row| row.payload))
    }

    pub fn upsert_pr(&self, pr: &PullRequestPayload) -> Result<()> {
        let row = PrRow::from(pr);
        let payload = encode(&row.payload)?;
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO pr_cache (id, repo_id, number, title, payload)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET title = excluded.title, payload = excluded.payload",
                rusqlite::params![row.id, row.repo_id, row.number, row.title, payload],
            )?;
            Ok(())
        })
    }

    /// Returns whether the cache now differs from what it held.
    pub fn replace_pull_requests(&self, prs: &[PullRequestPayload]) -> Result<bool> {
        let mut incoming: Vec<PrRow> = prs.iter().map(PrRow::from).collect();
        incoming.sort_by(|a, b| a.id.cmp(&b.id));
        let encoded = incoming
            .iter()
            .map(|row| encode(&row.payload))
            .collect::<Result<Vec<_>>>()?;
        self.with_tx(|conn| {
            let mut stored = all(conn)?;
            stored.sort_by(|a, b| a.id.cmp(&b.id));
            if stored == incoming {
                return Ok(false);
            }
            // One transaction so a concurrent snapshot never sees an empty table mid-sync.
            conn.execute("DELETE FROM pr_cache", [])?;
            for (row, payload) in incoming.iter().zip(&encoded) {
                conn.execute(
                    "INSERT INTO pr_cache (id, repo_id, number, title, payload)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![row.id, row.repo_id, row.number, row.title, payload],
                )?;
            }
            Ok(true)
        })
    }
}

fn encode(payload: &PullRequestPayload) -> Result<String> {
    serde_json::to_string(payload).map_err(|error| Error::Github(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::types::{PrChecksState, PrReviewState};

    fn pr(number: i64, title: &str) -> PullRequestPayload {
        PullRequestPayload {
            num: number,
            title: title.into(),
            author: "ana".into(),
            head: "feat".into(),
            base: "main".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            checks: PrChecksState::Pass,
            review: PrReviewState::Required,
            files: 1,
            is_draft: false,
            html_url: format!("https://github.com/o/r/pull/{number}"),
            repo_full_name: "o/r".into(),
            repo_id: None,
        }
    }

    #[test]
    fn an_unchanged_pr_list_reports_no_change() {
        let store = Store::in_memory().unwrap();
        assert!(store.replace_pull_requests(&[pr(1, "a")]).unwrap());
        assert!(!store.replace_pull_requests(&[pr(1, "a")]).unwrap());
        assert!(store.replace_pull_requests(&[pr(1, "b")]).unwrap());
        let cached = store.with_read(all).unwrap();
        assert_eq!(cached, [PrRow::from(&pr(1, "b"))]);
    }

    #[test]
    fn a_cached_pr_that_no_longer_reads_is_left_out() {
        let store = Store::in_memory().unwrap();
        store.upsert_pr(&pr(1, "a")).unwrap();
        store
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO pr_cache (id, number, title, payload) VALUES ('x#2', 2, 'x', '{}')",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        let ids: Vec<String> = store
            .with_read(all)
            .unwrap()
            .into_iter()
            .map(|row| row.id)
            .collect();
        assert_eq!(ids, ["o/r#1"]);
    }
}
