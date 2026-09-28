use crate::store::types::{ThreadRow, WorkspaceRow};

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
        .filter(|t| t.status == "running" || t.status == "provisioning")
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
        let text = fallback_summary(&ws, &[ThreadRow {
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
        }]);
        assert!(text.contains("#482"));
    }
}
