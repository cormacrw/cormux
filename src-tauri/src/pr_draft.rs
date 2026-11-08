use crate::error::Result;
use crate::git::WorktreeDiff;
use crate::llm::LlmClient;
use crate::store::types::WorkspaceRow;

/// A description built from what's known when the model can't be asked: the goal the thread
/// was started with (or the workspace name) and the first sentence of the card summary.
pub fn fallback_why(workspace: &WorkspaceRow, goal: &str) -> String {
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
        Ok(text) if !text.trim().is_empty() => Ok((text.trim().to_string(), true)),
        _ => Ok((fallback_why(workspace, goal), false)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace(name: &str) -> WorkspaceRow {
        WorkspaceRow {
            status: crate::model::WorkspaceLifecycle::Running,
            summary: Some("Adds idle timeout. Sessions are rejected after 30 minutes.".into()),
            ..crate::store::types::WorkspaceRow::new(
                crate::model::WorkspaceKind::Lead,
                "w1",
                "r1",
                name,
                "feat/x",
                "main",
                "/tmp/wt",
            )
        }
    }

    #[test]
    fn blank_prompt_falls_back_to_default() {
        assert_eq!(pr_prompt(None), DEFAULT_PR_PROMPT);
        assert_eq!(pr_prompt(Some("  \n".into())), DEFAULT_PR_PROMPT);
        assert_eq!(pr_prompt(Some(" Be brief. ".into())), "Be brief.");
    }

    /// The fallback only ever says what the thread and summary say; a workspace that happens
    /// to be named after a common topic gets no invented story about it.
    #[test]
    fn fallback_never_invents_details() {
        let text = fallback_why(&workspace("Auth session timeout"), "");
        assert_eq!(
            text,
            "This change is needed to auth session timeout. Adds idle timeout."
        );
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
