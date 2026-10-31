//! Names for workspaces created without one. A workspace gets a draft name from its prompt
//! straight away, then Haiku replaces it unless the user renamed it first. A workspace started
//! without a prompt is numbered. Branches work the same way, except Haiku's answer is taken
//! before the worktree is created, so the branch is never renamed under a running agent.

use std::time::Duration;

use crate::llm::LlmClient;

pub const WORKSPACE_NAME_MAX: usize = 48;

/// How long a new workspace waits on Haiku for a branch name before keeping the draft.
pub const BRANCH_NAME_TIMEOUT: Duration = Duration::from_secs(10);

const BRANCH_TYPES: [&str; 6] = ["feat", "fix", "refactor", "chore", "docs", "test"];
const BRANCH_SLUG_WORDS: usize = 5;

/// `Workspace N`, one past the highest number already used.
pub fn next_workspace_name<'a>(existing: impl Iterator<Item = &'a str>) -> String {
    let highest = existing
        .filter_map(|name| name.strip_prefix("Workspace ")?.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    format!("Workspace {}", highest + 1)
}

/// The prompt's first words, cut at a word boundary. Shown until Haiku answers.
pub fn draft_title(prompt: &str, max: usize) -> String {
    let trimmed = prompt
        .trim()
        .trim_end_matches(|c: char| c == '.' || c.is_whitespace());
    let mut out = String::new();
    for word in trimmed.split_whitespace() {
        let next_len = out.chars().count() + usize::from(!out.is_empty()) + word.chars().count();
        if next_len > max {
            break;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    if out.is_empty() {
        out = trimmed.chars().take(max).collect();
    }
    capitalize(&out)
}

pub fn build_title_prompt(prompt: &str, kind: &str) -> String {
    format!(
        "Write a short title (2 to 5 words) for a {kind} whose first request is below. \
         Sentence case, no quotes, no trailing punctuation, no markdown. Reply with the title only.\n\n\
         Request:\n{}",
        prompt.trim()
    )
}

/// Haiku's reply as a title, or `None` if it came back empty.
pub fn clean_title(raw: &str, max: usize) -> Option<String> {
    let line = raw.lines().map(str::trim).find(|line| !line.is_empty())?;
    let line = line
        .trim_start_matches("Title:")
        .trim()
        .trim_matches(['"', '\'', '`', '*', '#', '“', '”'])
        .trim_end_matches(['.', '!', ':'])
        .trim();
    if line.is_empty() {
        return None;
    }
    Some(
        capitalize(&line.chars().take(max).collect::<String>())
            .trim_end()
            .to_string(),
    )
}

pub async fn generate_title(
    llm: &LlmClient,
    prompt: &str,
    kind: &str,
    max: usize,
) -> Option<String> {
    match llm.complete(&build_title_prompt(prompt, kind)).await {
        Ok(result) => clean_title(&result.text, max),
        Err(error) => {
            log::warn!("could not name the {kind}: {error}");
            None
        }
    }
}

pub fn build_branch_prompt(prompt: &str) -> String {
    format!(
        "Write a git branch name for the task whose first request is below. \
         Use the form type/slug. type is one of {}: pick by what the work is, not by the words used. \
         slug is 2 to 4 lowercase words joined by dashes, naming the change itself, the way a person \
         would, not repeating the request. Example: feat/oauth-login. Reply with the branch name only.\n\n\
         Request:\n{}",
        BRANCH_TYPES.join(", "),
        prompt.trim()
    )
}

/// Haiku's reply as a branch name, or `None` if it isn't one.
pub fn clean_branch(raw: &str) -> Option<String> {
    let line = raw.lines().map(str::trim).find(|line| !line.is_empty())?;
    let line = line.trim_matches(['"', '\'', '`', '*']).to_lowercase();
    let (kind, slug) = line.split_once('/')?;
    if !BRANCH_TYPES.contains(&kind) {
        return None;
    }
    let words: Vec<&str> = slug
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .take(BRANCH_SLUG_WORDS)
        .collect();
    if words.is_empty() {
        return None;
    }
    Some(format!("{kind}/{}", words.join("-")))
}

pub async fn generate_branch(llm: &LlmClient, prompt: &str) -> Option<String> {
    match llm.complete(&build_branch_prompt(prompt)).await {
        Ok(result) => clean_branch(&result.text),
        Err(error) => {
            log::warn!("could not name the branch: {error}");
            None
        }
    }
}

/// `name`, or `name-2`, `name-3`… if that's taken.
pub fn unique_branch(name: &str, taken: &[String]) -> String {
    let mut candidate = name.to_string();
    let mut n = 2;
    while taken.contains(&candidate) {
        candidate = format!("{name}-{n}");
        n += 1;
    }
    candidate
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspaces_are_numbered_past_the_highest() {
        assert_eq!(next_workspace_name([].into_iter()), "Workspace 1");
        assert_eq!(
            next_workspace_name(["Workspace 2", "OAuth login", "Workspace 7"].into_iter()),
            "Workspace 8"
        );
    }

    #[test]
    fn draft_cuts_at_a_word_boundary() {
        assert_eq!(
            draft_title("fix the webhook signature check that fails on retries.", 24),
            "Fix the webhook"
        );
    }

    #[test]
    fn draft_keeps_a_long_first_word() {
        assert_eq!(draft_title("supercalifragilistic", 5), "Super");
    }

    #[test]
    fn clean_strips_quotes_and_punctuation() {
        assert_eq!(
            clean_title("\"OAuth login with Supabase.\"\n", 48).as_deref(),
            Some("OAuth login with Supabase")
        );
        assert_eq!(
            clean_title("Title: fix retries", 48).as_deref(),
            Some("Fix retries")
        );
    }

    #[test]
    fn clean_rejects_empty_replies() {
        assert_eq!(clean_title("  \n\"\"", 48), None);
    }

    #[test]
    fn clean_branch_tidies_the_reply() {
        assert_eq!(
            clean_branch("`feat/OAuth_login`\n").as_deref(),
            Some("feat/oauth-login")
        );
        assert_eq!(
            clean_branch("fix/a b c d e f g").as_deref(),
            Some("fix/a-b-c-d-e")
        );
    }

    #[test]
    fn clean_branch_rejects_anything_else() {
        assert_eq!(clean_branch("Here is a name: feat/oauth"), None);
        assert_eq!(clean_branch("oauth-login"), None);
        assert_eq!(clean_branch("wip/oauth"), None);
        assert_eq!(clean_branch("feat/---"), None);
        assert_eq!(clean_branch(""), None);
    }

    #[test]
    fn unique_branch_numbers_past_taken_names() {
        assert_eq!(unique_branch("feat/a", &[]), "feat/a");
        assert_eq!(
            unique_branch("feat/a", &["feat/a".into(), "feat/a-2".into()]),
            "feat/a-3"
        );
    }
}
