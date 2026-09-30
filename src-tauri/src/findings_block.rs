//! The Reviewer ends its final reply with its findings as JSON inside
//! `<cormux-findings>` tags. This module pulls that block out of the reply text.
//! The frontend hides the same block when it renders the conversation.

use serde::Deserialize;

pub const OPEN_TAG: &str = "<cormux-findings>";
pub const CLOSE_TAG: &str = "</cormux-findings>";

#[derive(Debug, Clone, PartialEq)]
pub struct ReviewFinding {
    pub severity: String,
    pub title: String,
    pub file: Option<String>,
    pub line: Option<i64>,
    pub explanation: String,
}

#[derive(Debug, PartialEq)]
pub enum BlockError {
    /// The reply has no findings block (or it was cut off before the closing tag).
    Missing,
    /// There is a block, but its contents aren't the JSON array we asked for.
    Invalid(String),
}

#[derive(Deserialize)]
struct RawFinding {
    #[serde(default)]
    severity: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    file: Option<String>,
    #[serde(default)]
    line: Option<serde_json::Value>,
    #[serde(default, alias = "body", alias = "description")]
    explanation: String,
}

/// Parses the last findings block in `reply`. `worktree` is used to turn absolute
/// paths the agent may have written back into repo-relative ones.
pub fn parse_findings_block(
    reply: &str,
    worktree: &str,
) -> Result<Vec<ReviewFinding>, BlockError> {
    let start = reply.rfind(OPEN_TAG).ok_or(BlockError::Missing)?;
    let body_start = start + OPEN_TAG.len();
    let body_len = reply[body_start..]
        .find(CLOSE_TAG)
        .ok_or(BlockError::Missing)?;
    let body = strip_code_fence(reply[body_start..body_start + body_len].trim());

    let raw: Vec<RawFinding> =
        serde_json::from_str(body).map_err(|error| BlockError::Invalid(error.to_string()))?;
    Ok(raw
        .into_iter()
        .filter_map(|item| normalize(item, worktree))
        .collect())
}

/// Agents often wrap JSON in a ```json fence even when asked not to.
fn strip_code_fence(body: &str) -> &str {
    let Some(rest) = body.strip_prefix("```") else {
        return body;
    };
    let rest = rest.split_once('\n').map_or("", |(_, after)| after);
    rest.trim_end().strip_suffix("```").unwrap_or(rest).trim()
}

fn normalize(item: RawFinding, worktree: &str) -> Option<ReviewFinding> {
    let title = item.title.trim().to_string();
    if title.is_empty() {
        return None;
    }
    let severity = match item.severity.trim().to_ascii_lowercase().as_str() {
        "blocking" | "blocker" | "critical" | "high" => "blocking",
        "nit" | "nitpick" | "low" => "nit",
        _ => "suggestion",
    }
    .to_string();
    let file = item
        .file
        .map(|path| relative_path(path.trim(), worktree))
        .filter(|path| !path.is_empty());
    let line = match item.line {
        Some(serde_json::Value::Number(number)) => number.as_i64(),
        Some(serde_json::Value::String(text)) => text.trim().parse().ok(),
        _ => None,
    }
    .filter(|line| *line >= 1 && file.is_some());
    Some(ReviewFinding {
        severity,
        title,
        file,
        line,
        explanation: item.explanation.trim().to_string(),
    })
}

fn relative_path(path: &str, worktree: &str) -> String {
    let worktree = worktree.trim_end_matches('/');
    let path = if !worktree.is_empty() {
        path.strip_prefix(worktree)
            .map(|rest| rest.trim_start_matches('/'))
            .unwrap_or(path)
    } else {
        path
    };
    path.trim_start_matches("./").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORKTREE: &str = "/Users/me/.harness/worktrees/app/feat-x";

    #[test]
    fn parses_block_at_end_of_reply() {
        let reply = r#"Overall this looks good, two things to fix.

<cormux-findings>
[
  {"severity": "blocking", "title": "Retries can double-process", "file": "src/retry.ts", "line": 48, "explanation": "Guard with the event id."},
  {"severity": "nit", "title": "Typo", "file": null, "line": null, "explanation": "recieve → receive"}
]
</cormux-findings>"#;
        let findings = parse_findings_block(reply, WORKTREE).unwrap();
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].severity, "blocking");
        assert_eq!(findings[0].file.as_deref(), Some("src/retry.ts"));
        assert_eq!(findings[0].line, Some(48));
        assert_eq!(findings[1].file, None);
        assert_eq!(findings[1].line, None);
    }

    #[test]
    fn empty_array_means_no_findings() {
        let reply = "Nothing to flag.\n<cormux-findings>[]</cormux-findings>";
        assert_eq!(parse_findings_block(reply, WORKTREE), Ok(vec![]));
    }

    #[test]
    fn tolerates_fences_absolute_paths_and_loose_fields() {
        let reply = format!(
            "<cormux-findings>\n```json\n[{{\"severity\":\"High\",\"title\":\" Leak \",\"file\":\"{WORKTREE}/src/a.rs\",\"line\":\"12\",\"body\":\"Close it.\"}},{{\"severity\":\"?\",\"title\":\"Maybe\",\"file\":\"./b.rs\",\"line\":0}},{{\"title\":\"\"}}]\n```\n</cormux-findings>"
        );
        let findings = parse_findings_block(&reply, WORKTREE).unwrap();
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].severity, "blocking");
        assert_eq!(findings[0].title, "Leak");
        assert_eq!(findings[0].file.as_deref(), Some("src/a.rs"));
        assert_eq!(findings[0].line, Some(12));
        assert_eq!(findings[0].explanation, "Close it.");
        assert_eq!(findings[1].severity, "suggestion");
        assert_eq!(findings[1].file.as_deref(), Some("b.rs"));
        assert_eq!(findings[1].line, None);
    }

    #[test]
    fn uses_the_last_block_when_the_reply_quotes_the_format() {
        let reply = "I'll finish with <cormux-findings>[example]</cormux-findings> as asked.\n<cormux-findings>[]</cormux-findings>";
        assert_eq!(parse_findings_block(reply, WORKTREE), Ok(vec![]));
    }

    #[test]
    fn reports_missing_and_invalid_blocks() {
        assert_eq!(
            parse_findings_block("Review done.", WORKTREE),
            Err(BlockError::Missing)
        );
        assert_eq!(
            parse_findings_block("<cormux-findings>[{\"title\":", WORKTREE),
            Err(BlockError::Missing)
        );
        assert!(matches!(
            parse_findings_block("<cormux-findings>not json</cormux-findings>", WORKTREE),
            Err(BlockError::Invalid(_))
        ));
    }
}
