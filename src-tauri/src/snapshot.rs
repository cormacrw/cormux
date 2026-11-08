//! What the webview loads in one go: each feature's rows, read together, plus the live state
//! only memory holds.

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

use crate::app::WorkspaceAppRuntime;
use crate::approvals::ApprovalRow;
use crate::entities::EntityRow;
use crate::error::Result;
use crate::findings::FindingRow;
use crate::github::PrRow;
use crate::harness_config::{RepoRunCommand, WorkspaceRunCommand};
use crate::metrics::MemorySample;
use crate::review::ReviewRow;
use crate::scratch::ScratchRow;
use crate::settings::Settings;
use crate::state::AppState;
use crate::store::types::{RepoRecord, ThreadRow, WorkspaceRow};
use crate::transcript::ThreadActivityRow;
use crate::workspace_git::WorkspaceGitRuntime;
use crate::workspaces::WorkspaceRecord;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PersistedSnapshot {
    pub reviews: Vec<ReviewRow>,
    pub repos: Vec<RepoRecord>,
    pub workspaces: Vec<WorkspaceRow>,
    pub threads: Vec<ThreadRow>,
    /// Newest first.
    pub scratches: Vec<ScratchRow>,
    /// When each thread last moved; threads that never have are left out.
    pub thread_activity: Vec<ThreadActivityRow>,
    pub approvals: Vec<ApprovalRow>,
    pub findings: Vec<FindingRow>,
    /// The open threads' attached entities, oldest first.
    pub entities: Vec<EntityRow>,
    pub pull_requests: Vec<PrRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub version: u64,
    pub settings: Settings,
    pub persisted: PersistedSnapshot,
    /// Every live workspace, in the order they were created.
    pub workspaces: Vec<WorkspaceRecord>,
    pub workspace_git: Vec<WorkspaceGitRuntime>,
    pub memory: Option<MemorySample>,
    /// Approvals waiting on the user, which the dock badge also shows.
    pub pending_approvals: usize,
    /// True when the last PR sync was signed in to `gh`, or, before one has run, when an earlier
    /// sync's time is stored.
    pub github_auth_configured: bool,
    /// True when a ClickUp API key is saved. Every ClickUp surface hides without one.
    pub clickup_configured: bool,
    pub workspace_apps: Vec<WorkspaceAppRuntime>,
    /// Each repo's run command (its own, else its committed config's), for repos with one.
    pub repo_run_commands: Vec<RepoRunCommand>,
    /// What Run starts in each live workspace (its own, else its repo's), for those with one.
    pub workspace_run_commands: Vec<WorkspaceRunCommand>,
    /// Repos Remove would refuse now, and why.
    pub repo_removal_blocks: Vec<crate::repos::RepoRemovalBlock>,
    /// Workspaces whose branch can't be switched now, and why.
    pub workspace_branch_locks: Vec<crate::workspaces::WorkspaceBranchLock>,
}

fn persisted(store: &crate::store::Store) -> Result<PersistedSnapshot> {
    store.read_consistent(|conn| {
        Ok(PersistedSnapshot {
            reviews: crate::review::live_in(conn)?,
            repos: crate::store::repos_in(conn)?,
            workspaces: crate::store::live_workspaces_in(conn)?,
            threads: crate::store::workspace_threads_in(conn)?,
            scratches: crate::scratch::all(conn)?,
            thread_activity: crate::transcript::activity_in(conn)?,
            approvals: crate::approvals::visible_in(conn)?,
            findings: crate::findings::live_in(conn)?,
            entities: crate::entities::visible(conn)?,
            pull_requests: crate::github::pull_requests_in(conn)?,
        })
    })
}

#[tauri::command]
#[specta::specta]
pub async fn get_snapshot(state: State<'_, AppState>) -> Result<Snapshot> {
    let (settings, persisted) = state
        .store
        .blocking(|store| Ok((Settings::read(store)?, persisted(store)?)))
        .await?;
    let workspaces = state
        .workspaces
        .records_for(&persisted.workspaces, &persisted.repos)
        .await;
    let workspace_ids: Vec<String> = workspaces.iter().map(|row| row.id.clone()).collect();
    let workspace_git = state.workspace_git.snapshot(&workspace_ids).await;
    let repo_run_commands = crate::harness_config::repo_run_commands(&persisted.repos);
    let repo_removal_blocks = crate::repos::removal_blocks(&persisted.repos, &persisted.workspaces);
    let workspace_run_commands =
        crate::harness_config::workspace_run_commands(&persisted.workspaces, &repo_run_commands);
    let workspace_branch_locks = crate::workspaces::branch_locks(&workspaces, &persisted.threads);

    Ok(Snapshot {
        version: state.out.version(),
        settings,
        persisted,
        workspaces,
        workspace_git,
        memory: state.metrics.latest(),
        pending_approvals: state.approvals.pending_count(),
        github_auth_configured: state.pr_sync.auth_configured(&state),
        clickup_configured: state.clickup.is_configured(),
        workspace_apps: state.apps.snapshot(),
        repo_run_commands,
        workspace_run_commands,
        repo_removal_blocks,
        workspace_branch_locks,
    })
}

#[cfg(test)]
pub fn read(store: &crate::store::Store) -> PersistedSnapshot {
    persisted(store).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::FindingRow;
    use crate::model::{ApprovalPayload, ThreadStatus, WorkspaceLifecycle};
    use crate::store::Store;

    #[test]
    fn lists_what_the_live_workspaces_and_visible_threads_hold() {
        let store = Store::in_memory().unwrap();
        store.seed_workspace("w1", WorkspaceLifecycle::Ready);
        store
            .set_review_status("w1", crate::review::ReviewStatus::Ready)
            .unwrap();
        store.seed_thread("t1", "w1", ThreadStatus::Idle);
        store.seed_workspace("gone", WorkspaceLifecycle::Idle);
        store.seed_thread("t2", "gone", ThreadStatus::Idle);
        for thread in ["t1", "t2"] {
            store
                .append_event(thread, "message", "{\"role\":\"user\",\"text\":\"hi\"}")
                .unwrap();
            store
                .insert_pending_approval(
                    &format!("a-{thread}"),
                    thread,
                    "edit",
                    &ApprovalPayload::titled("Edit"),
                )
                .unwrap();
            store.attach_entity(thread, "PR", "https://x").unwrap();
        }
        store
            .upsert_finding(&FindingRow {
                id: "f1".into(),
                workspace_id: "w1".into(),
                severity: "high".into(),
                title: "Leak".into(),
                file: Some("a.ts".into()),
                line: Some(10),
                explanation: "secret".into(),
                status: crate::model::FindingStatus::Open,
                commit_sha: None,
                sent_to_thread_id: None,
            })
            .unwrap();
        store.archive_workspace("gone").unwrap();

        let snap = read(&store);
        assert_eq!(snap.reviews.len(), 1);
        assert_eq!(
            snap.reviews[0].status,
            Some(crate::review::ReviewStatus::Ready)
        );
        assert_eq!(snap.repos[0].id, "r1");
        let ids = |rows: Vec<String>| rows;
        assert_eq!(
            ids(snap.workspaces.iter().map(|w| w.id.clone()).collect()),
            ["w1"]
        );
        assert_eq!(
            ids(snap.threads.iter().map(|t| t.id.clone()).collect()),
            ["t1"]
        );
        assert_eq!(
            ids(snap.approvals.iter().map(|a| a.id.clone()).collect()),
            ["a-t1"]
        );
        assert_eq!(
            ids(snap.entities.iter().map(|e| e.thread_id.clone()).collect()),
            ["t1"]
        );
        assert_eq!(
            ids(snap
                .thread_activity
                .iter()
                .map(|a| a.thread_id.clone())
                .collect()),
            ["t1"]
        );
        assert_eq!(snap.findings[0].title, "Leak");
    }

    /// `cargo test --lib snapshot_timing -- --ignored --nocapture`: snapshot and paging cost on
    /// a store the size of a busy week (50 threads × 5,000 events, one thread mid-turn).
    #[test]
    #[ignore = "benchmark; prints timings"]
    fn snapshot_timing() {
        use std::time::Instant;
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("bench.db"), &[]).unwrap();
        store
            .with_conn(|conn| {
                conn.execute_batch("PRAGMA foreign_keys = OFF;")?;
                let tx = conn.unchecked_transaction()?;
                for t in 0..50 {
                    let id = format!("t{t}");
                    tx.execute(
                        "INSERT INTO workspaces (id, repo_id, name, branch, worktree_path, status)
                         VALUES (?1, 'r', ?1, ?1, '/tmp', 'idle')",
                        [&id],
                    )?;
                    tx.execute(
                        "INSERT INTO threads (id, workspace_id, title, engine, status)
                         VALUES (?1, ?1, 'Lead', 'claude', 'idle')",
                        [&id],
                    )?;
                    let mut stmt = tx.prepare_cached(
                        "INSERT INTO thread_events (thread_id, seq, kind, payload)
                         VALUES (?1, ?2, ?3, ?4)",
                    )?;
                    let total = if t == 0 { 7000 } else { 5000 };
                    for seq in 0..total {
                        let (kind, payload) = match seq % 1000 {
                            0 => ("message", r#"{"role":"user","text":"go"}"#),
                            999 if seq < 5000 => ("turn_end", "{}"),
                            n if n % 3 == 0 => ("tool", r#"{"type":"toolCall","title":"Read"}"#),
                            _ => (
                                "message",
                                r#"{"type":"messageChunk","role":"agent","text":"a streamed chunk"}"#,
                            ),
                        };
                        stmt.execute(rusqlite::params![id, seq + 1, kind, payload])?;
                    }
                }
                tx.commit()?;
                Ok(())
            })
            .unwrap();
        for label in ["cold", "warm", "warm"] {
            let start = Instant::now();
            read(&store);
            eprintln!("{label} snapshot: {:?}", start.elapsed());
        }
        let start = Instant::now();
        for _ in 0..20 {
            store
                .thread_event_page("t1", None, None, Some(2000))
                .unwrap();
        }
        eprintln!("20 pages of 2000: {:?}", start.elapsed());
        let start = Instant::now();
        for i in 0..200 {
            store
                .append_event("t1", "tool", &format!("{{\"n\":{i}}}"))
                .unwrap();
        }
        eprintln!("200 appends: {:?}", start.elapsed());
    }
}
