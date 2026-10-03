use crate::error::Result;
use crate::git::WorktreeDiff;
use crate::llm::LlmClient;
use crate::store::types::{ThreadEventRow, WorkspaceRow};

const CANNED: &[(&str, &str)] = &[
    (
        "auth session timeout",
        "Sessions never expire while the cookie is valid, so a forgotten or stolen session stays usable indefinitely. This adds a 30-minute sliding idle timeout and a 12-hour absolute cap: stale sessions are rejected in getSession() and the user is sent back to /login.",
    ),
    (
        "stripe webhook",
        "Stripe signature checks are failing because the webhook parses the JSON body before verifying it, which changes the signed payload. Verifying against the raw request body lets legitimate events through again and keeps forged ones out.",
    ),
    (
        "dashboard perf",
        "The /dashboard first paint waits on every chart widget loading, even ones below the fold. Lazy-loading the charts lets the page render sooner without changing what users see once it settles.",
    ),
];

pub fn fallback_why(workspace: &WorkspaceRow, goal: &str) -> String {
    let name_key = workspace.name.to_lowercase();
    for (needle, text) in CANNED {
        if name_key.contains(needle) {
            return (*text).into();
        }
    }

    let goal_trimmed = goal.trim().trim_end_matches('.').trim();
    let goal_source = if goal_trimmed.is_empty() {
        workspace.name.trim().trim_end_matches('.').trim()
    } else {
        goal_trimmed
    };
    let mut goal_lower = goal_source.to_string();
    if let Some(first) = goal_lower.chars().next() {
        goal_lower = first.to_lowercase().collect::<String>() + &goal_lower[first.len_utf8()..];
    }

    let summary_sentence = workspace
        .summary
        .as_deref()
        .and_then(|summary| summary.split(". ").next())
        .map(|sentence| sentence.trim_end_matches('.').to_string())
        .filter(|sentence| !sentence.is_empty());

    match summary_sentence {
        Some(summary) => format!("This change is needed to {goal_lower}. {summary}."),
        None => format!("This change is needed to {goal_lower}."),
    }
}

/// Settings key for the instructions the drafting model gets; empty means the default.
pub const PR_PROMPT_KEY: &str = "prPrompt";

pub const DEFAULT_PR_PROMPT: &str = "Write the description for this pull request in GitHub-flavoured markdown. \
Open with two or three sentences on why the change is necessary: the problem it solves and why it matters now. \
Then add a \"## What changed\" section with a short bullet list of the meaningful changes, \
and a \"## How it was tested\" section if the thread shows any testing. \
Don't add a title, and don't invent details the thread and diff don't support.";

/// The saved prompt, or the default when it's unset or blank.
pub fn pr_prompt(saved: Option<String>) -> String {
    saved
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| DEFAULT_PR_PROMPT.into())
}

pub fn build_draft_prompt(
    instructions: &str,
    workspace: &WorkspaceRow,
    base: &str,
    goal: &str,
    transcript: &str,
    diff: &WorktreeDiff,
) -> String {
    let file_lines: Vec<String> = diff
        .files
        .iter()
        .take(40)
        .map(|file| format!("- {} (+{} −{})", file.path, file.added, file.deleted))
        .collect();
    let file_block = if file_lines.is_empty() {
        "No file changes yet.".into()
    } else {
        file_lines.join("\n")
    };

    format!(
        "{instructions}\n\n\
         Reply with only the description.\n\n\
         Workspace: \"{}\" on branch {} → base {}.\n\
         User goal: {}\n\
         Thread excerpt:\n{}\n\
         Changed files:\n{}",
        workspace.name,
        workspace.branch,
        base,
        if goal.is_empty() { "(unknown)" } else { goal },
        transcript,
        file_block,
    )
}

pub fn extract_goal_from_events(events: &[ThreadEventRow]) -> String {
    for row in events {
        if row.kind != "message" {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&row.payload)
            && value.get("role").and_then(|role| role.as_str()) == Some("user")
            && let Some(text) = value.get("text").and_then(|text| text.as_str())
        {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    String::new()
}

pub async fn draft_why(
    llm: &LlmClient,
    instructions: &str,
    workspace: &WorkspaceRow,
    base: &str,
    goal: &str,
    transcript: &str,
    diff: &WorktreeDiff,
) -> Result<(String, bool)> {
    let prompt = build_draft_prompt(instructions, workspace, base, goal, transcript, diff);
    match llm.complete(&prompt).await {
        Ok(result) if !result.text.trim().is_empty() => Ok((result.text.trim().to_string(), true)),
        _ => Ok((fallback_why(workspace, goal), false)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace(name: &str) -> WorkspaceRow {
        WorkspaceRow {
            id: "w1".into(),
            repo_id: "r1".into(),
            name: name.into(),
            branch: "feat/x".into(),
            worktree_path: "/tmp/wt".into(),
            status: "running".into(),
            created_at: String::new(),
            summary: Some("Adds idle timeout. Sessions are rejected after 30 minutes.".into()),
            summary_at: None,
            summary_source: "Haiku 4.5".into(),
            kind: None,
            pr_number: None,
            pr_html_url: None,
            modified_files: 2,
            archived_at: None,
        }
    }

    #[test]
    fn blank_prompt_falls_back_to_default() {
        assert_eq!(pr_prompt(None), DEFAULT_PR_PROMPT);
        assert_eq!(pr_prompt(Some("  \n".into())), DEFAULT_PR_PROMPT);
        assert_eq!(pr_prompt(Some(" Be brief. ".into())), "Be brief.");
    }

    #[test]
    fn fallback_uses_canned_auth_copy() {
        let text = fallback_why(&workspace("Auth session timeout"), "");
        assert!(text.contains("30-minute"));
    }

    #[test]
    fn fallback_builds_from_goal_and_summary() {
        let text = fallback_why(
            &workspace("Widget polish"),
            "Fix the broken checkout button",
        );
        assert!(text.contains("fix the broken checkout button"));
        assert!(text.contains("Adds idle timeout"));
    }
}
