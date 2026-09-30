//! Titles for workspaces and scratches created without one. A scratch gets a draft title from
//! the prompt straight away, then Haiku replaces it unless the user renamed it first. A workspace
//! is numbered.

use crate::llm::LlmClient;

pub const UNTITLED_SCRATCH: &str = "Untitled scratch";

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
}
