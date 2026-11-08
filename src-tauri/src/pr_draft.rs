use crate::error::Result;
use crate::git::WorktreeDiff;
use crate::llm::LlmClient;
use crate::store::types::WorkspaceRow;

/// A description built from what's known when the model can't be asked: the goal the thread
/// was started with, or the workspace name.
fn fallback_why(workspace: &WorkspaceRow, goal: &str) -> String {
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
    format!("This change is needed to {goal_lower}.")
}

pub const DEFAULT_PR_PROMPT: &str = "Write the description for this pull request in GitHub-flavoured markdown. \
Open with two or three sentences on why the change is necessary: the problem it solves and why it matters now. \
Then add a \"## What changed\" section with a short bullet list of the meaningful changes, \
and a \"## How it was tested\" section if the thread shows any testing. \
Don't add a title, and don't invent details the thread and diff don't support.";

pub fn pr_prompt(saved: &str) -> String {
    match saved.trim() {
        "" => DEFAULT_PR_PROMPT.into(),
        text => text.into(),
    }
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
            summary: Some("Started “Auth” on a branch from main.".into()),
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
        assert_eq!(pr_prompt(""), DEFAULT_PR_PROMPT);
        assert_eq!(pr_prompt("  \n"), DEFAULT_PR_PROMPT);
        assert_eq!(pr_prompt(" Be brief. "), "Be brief.");
    }

    /// The fallback only says what the goal says, never the intro Cormux wrote; a workspace
    /// named after a common topic gets no invented story about it.
    #[test]
    fn fallback_says_only_the_goal_or_the_name() {
        assert_eq!(
            fallback_why(&workspace("Auth session timeout"), ""),
            "This change is needed to auth session timeout."
        );
        assert_eq!(
            fallback_why(
                &workspace("Widget polish"),
                "Fix the broken checkout button."
            ),
            "This change is needed to fix the broken checkout button."
        );
    }
}
