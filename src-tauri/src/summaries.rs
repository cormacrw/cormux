use crate::error::{Error, Result};
use crate::ipc::types::WorkspaceSummaryResult;
use crate::state::AppState;
use crate::store::types::{ThreadRow, WorkspaceRow};
use crate::workspace::ThreadStatus;

pub const SUMMARY_MODEL_LABEL: &str = "Haiku 4.5";

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
    if workspace.kind.as_deref() == Some("review") {
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

/// A cached workspace card summary, or a new one (debounced LLM + local fallback).
pub async fn summarise(state: &AppState, workspace_id: String) -> Result<WorkspaceSummaryResult> {
    let workspace = state
        .store
        .workspace_by_id(&workspace_id)?
        .filter(|row| row.archived_at.is_none())
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;

    if let (Some(summary), Some(at)) = (&workspace.summary, &workspace.summary_at)
        && !summary.is_empty()
    {
        return Ok(WorkspaceSummaryResult {
            workspace_id,
            summary: summary.clone(),
            summary_at: at.clone(),
            summary_source: workspace.summary_source.clone(),
            from_llm: true,
        });
    }

    let threads = state.store.threads_for_workspace(&workspace_id)?;

    let prompt = build_summary_prompt(&workspace, &threads);
    let (summary, from_llm) = match state.llm.summarise(&workspace_id, &prompt).await {
        Ok(Some(result)) if !result.text.is_empty() => (result.text, true),
        _ => (fallback_summary(&workspace, &threads), false),
    };

    state
        .store
        .set_workspace_summary(&workspace_id, &summary, SUMMARY_MODEL_LABEL)?;

    let updated = state
        .store
        .workspace_by_id(&workspace_id)?
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;

    Ok(WorkspaceSummaryResult {
        workspace_id,
        summary,
        summary_at: updated.summary_at.unwrap_or_else(|| "now".into()),
        summary_source: updated.summary_source,
        from_llm,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace() -> WorkspaceRow {
        WorkspaceRow {
            id: "w1".into(),
            repo_id: "r1".into(),
            name: "Auth session timeout".into(),
            branch: "feat/auth".into(),
            worktree_path: "/tmp/wt".into(),
            status: "running".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            summary: None,
            summary_at: None,
            summary_source: SUMMARY_MODEL_LABEL.into(),
            kind: None,
            pr_number: None,
            pr_html_url: None,
            modified_files: 3,
            archived_at: None,
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
        let mut ws = workspace();
        ws.kind = Some("review".into());
        ws.pr_number = Some(482);
        let text = fallback_summary(
            &ws,
            &[ThreadRow {
                id: "t1".into(),
                workspace_id: "w1".into(),
                title: "Reviewer".into(),
                engine: "claude".into(),
                session_id: None,
                status: "running".into(),
                used_tokens: None,
                context_size: None,
                cost_usd: None,
                transcript_readonly: false,
                retired: false,
                parent_thread_id: None,
            }],
        );
        assert!(text.contains("#482"));
    }
}
