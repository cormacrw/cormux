//! Workspace card summaries: Haiku's, at most once a minute per workspace, and the ones Cormux
//! writes itself while there's nothing for Haiku to say yet or Haiku can't be reached.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{Error, Result};
use crate::model::{SUMMARY_MODEL_LABEL, ThreadStatus, WorkspaceKind};
use crate::state::AppState;
use crate::store::types::{ThreadRow, WorkspaceRow};

/// A workspace asks Haiku at most this often; in between it gets the summary it has.
const DEBOUNCE: Duration = Duration::from_secs(60);

/// The source of a summary Cormux wrote itself rather than Haiku.
pub const LOCAL_SOURCE: &str = "Cormux";

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSummaryResult {
    pub workspace_id: String,
    pub summary: String,
    pub summary_at: String,
    pub summary_source: String,
    pub from_llm: bool,
}

/// When each workspace last asked Haiku.
#[derive(Default)]
pub struct Summaries {
    last_asked: Mutex<HashMap<String, Instant>>,
}

impl Summaries {
    /// Whether the workspace may ask Haiku now; if so, the next minute's asks are spent.
    fn may_ask(&self, workspace_id: &str) -> bool {
        let mut last = self
            .last_asked
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if last
            .get(workspace_id)
            .is_some_and(|at| at.elapsed() < DEBOUNCE)
        {
            return false;
        }
        last.insert(workspace_id.to_string(), Instant::now());
        true
    }
}

/// What a new Lead's card says until a real summary is written.
pub fn lead_started(name: &str, base: &str) -> String {
    format!(
        "Just started on “{name}”, branched from {base}. Claude Code is reading the repo to find where the change belongs; no files have been touched yet."
    )
}

/// What a new PR review's card says until the review is ready.
pub fn review_started(pr_label: &str, author: &str, base: &str) -> String {
    format!(
        "Reviewing {pr_label} from {author} against {base}. Claude Code is reading the diff and will draft comments for your approval; nothing has been posted to GitHub yet."
    )
}

/// A finished PR review's card. `findings` reads like "2 blocking issues, …", or is `None`.
pub fn review_ready(pr_label: &str, author: &str, findings: Option<&str>) -> String {
    match findings {
        None => format!("Reviewed {pr_label} from {author}: no findings."),
        Some(findings) => format!(
            "Reviewed {pr_label} from {author}: {findings}. Choose which to send back for fixes in the Findings tab; nothing has been posted to GitHub yet."
        ),
    }
}

pub fn build_summary_prompt(workspace: &WorkspaceRow, threads: &[ThreadRow]) -> String {
    let thread_lines: Vec<String> = threads
        .iter()
        .map(|thread| {
            format!(
                "- {} ({}, status: {})",
                thread.title, thread.engine, thread.status
            )
        })
        .collect();
    format!(
        "Write at most three short sentences summarizing where this software workspace stands. \
         No markdown. Workspace: \"{}\" on branch {} (status: {}). \
         {} agent thread(s):\n{}\nModified files: {}.",
        workspace.name,
        workspace.branch,
        workspace.status,
        threads.len(),
        thread_lines.join("\n"),
        workspace.modified_files
    )
}

pub fn fallback_summary(workspace: &WorkspaceRow, threads: &[ThreadRow]) -> String {
    if threads.is_empty() {
        return format!(
            "Just started on “{}”, branched as {}. No agents are attached yet.",
            workspace.name, workspace.branch
        );
    }
    if workspace.kind == WorkspaceKind::PrReview {
        let pr = workspace
            .pr_number
            .map(|n| format!("#{n}"))
            .unwrap_or_else(|| "the PR".into());
        return format!(
            "Reviewing {pr} on branch {}. {} agent(s) are reading the diff and will draft comments for your approval; nothing has been posted yet.",
            workspace.branch,
            threads.len()
        );
    }
    let active = threads
        .iter()
        .filter(|t| t.has_status(ThreadStatus::Running) || t.has_status(ThreadStatus::Provisioning))
        .count();
    if active > 0 {
        format!(
            "Work continues on “{}” ({}). {} of {} agent(s) are active; {} file(s) touched so far.",
            workspace.name,
            workspace.branch,
            active,
            threads.len(),
            workspace.modified_files
        )
    } else {
        format!(
            "“{}” on {} is waiting on you or idle. {} agent(s) attached; {} modified file(s).",
            workspace.name,
            workspace.branch,
            threads.len(),
            workspace.modified_files
        )
    }
}

fn stored(workspace: &WorkspaceRow) -> Option<WorkspaceSummaryResult> {
    let summary = workspace.summary.as_ref().filter(|text| !text.is_empty())?;
    Some(WorkspaceSummaryResult {
        workspace_id: workspace.id.clone(),
        summary: summary.clone(),
        summary_at: workspace.summary_at.clone()?,
        summary_source: workspace.summary_source.clone(),
        from_llm: workspace.summary_source == SUMMARY_MODEL_LABEL,
    })
}

/// The workspace's card summary: Haiku's new one when the workspace may ask again, else the
/// one it has. Without Haiku, Cormux writes one from what it knows.
pub async fn summarise(state: &AppState, workspace_id: String) -> Result<WorkspaceSummaryResult> {
    let workspace = state
        .store
        .workspace_by_id(&workspace_id)?
        .filter(|row| row.archived_at.is_none())
        .ok_or_else(|| Error::unknown_workspace(&workspace_id))?;

    let may_ask = state.summaries.may_ask(&workspace_id);
    if !may_ask && let Some(result) = stored(&workspace) {
        return Ok(result);
    }

    let threads = state.store.threads_for_workspace(&workspace_id)?;
    let asked = if may_ask {
        state
            .llm
            .complete(&build_summary_prompt(&workspace, &threads))
            .await
    } else {
        Err(Error::Llm("asked within the last minute".into()))
    };
    let (summary, source) = match asked {
        Ok(text) if !text.trim().is_empty() => (text, SUMMARY_MODEL_LABEL),
        Ok(_) => (fallback_summary(&workspace, &threads), LOCAL_SOURCE),
        Err(error) => {
            log::debug!("summary for {workspace_id} without Haiku: {error}");
            (fallback_summary(&workspace, &threads), LOCAL_SOURCE)
        }
    };
    state
        .workspaces
        .set_summary(&workspace_id, &summary, source)?;

    let updated = state
        .store
        .workspace_by_id(&workspace_id)?
        .ok_or_else(|| Error::unknown_workspace(&workspace_id))?;
    stored(&updated).ok_or_else(|| Error::Store(format!("summary for {workspace_id} not saved")))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::model::{ThreadRole, WorkspaceLifecycle};

    fn workspace() -> WorkspaceRow {
        WorkspaceRow {
            modified_files: 3,
            ..WorkspaceRow::new(
                WorkspaceKind::Lead,
                "w1",
                "r1",
                "Auth session timeout",
                "feat/auth",
                "main",
                "/tmp/wt",
            )
            .with_status(WorkspaceLifecycle::Running)
        }
    }

    #[test]
    fn fallback_mentions_workspace_and_branch() {
        let text = fallback_summary(&workspace(), &[]);
        assert!(text.contains("Auth session timeout"));
        assert!(text.contains("feat/auth"));
    }

    #[test]
    fn review_fallback_mentions_pr() {
        let ws = WorkspaceRow {
            kind: WorkspaceKind::PrReview,
            ..workspace().with_pr(482, None)
        };
        let reviewer = ThreadRow::new("t1", "w1", ThreadRole::Reviewer, "claude")
            .with_status(ThreadStatus::Running);
        let text = fallback_summary(&ws, &[reviewer]);
        assert!(text.contains("#482"));
    }

    #[test]
    fn a_workspace_asks_at_most_once_a_minute() {
        let summaries = Summaries::default();
        assert!(summaries.may_ask("ws-1"));
        assert!(!summaries.may_ask("ws-1"));
        assert!(summaries.may_ask("ws-2"));
    }

    #[tokio::test]
    async fn a_seeded_summary_is_replaced_and_a_local_one_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let (state, _recording) =
            AppState::recorded(crate::paths::DataDir::new(dir.path().to_path_buf()));
        state
            .store
            .upsert_repo(&crate::store::types::RepoRecord {
                id: "r1".into(),
                path: "/tmp/app".into(),
                name: "app".into(),
                default_branch: Some("main".into()),
                setup_commands: String::new(),
                run_command: None,
                single_instance: false,
            })
            .unwrap();
        state.store.insert_workspace(&workspace()).unwrap();
        state
            .workspaces
            .set_summary("w1", &lead_started("Auth", "main"), LOCAL_SOURCE)
            .unwrap();
        // No `claude` on this PATH, so Haiku can't answer.
        *state.shell_env.write().await = crate::shell_env::ShellEnv::from_vars([(
            "PATH".to_string(),
            "/nonexistent".to_string(),
        )]);

        let first = summarise(&state, "w1".into()).await.unwrap();
        assert_eq!(first.summary, fallback_summary(&workspace(), &[]));
        assert!(!first.from_llm);
        assert_eq!(first.summary_source, LOCAL_SOURCE);

        // Within the minute the same summary comes back, still not Haiku's.
        let again = summarise(&state, "w1".into()).await.unwrap();
        assert_eq!(again.summary, first.summary);
        assert!(!again.from_llm);
    }
}
