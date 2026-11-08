//! Claude's tools by name: what each one does, and how to say it in words. Nothing outside this
//! adapter knows a Claude tool name.

use serde_json::Value;

use crate::engines::events::{EditVerb, ToolRole};

/// Asks the user something, so it shows as a question card.
pub const QUESTION_TOOL: &str = "AskUserQuestion";
/// Asks to start on the plan, so it shows as the plan's approval.
const EXIT_PLAN_TOOL: &str = "ExitPlanMode";
/// Nothing in the app could take a session back out of plan mode.
pub const ENTER_PLAN_TOOL: &str = "EnterPlanMode";
/// Keeps the session's todo list, which the thread shows as its plan.
pub const TODO_TOOL: &str = "TodoWrite";

/// Tools the thread draws as something else (a question card, the plan's approval), not a step.
pub(super) fn is_hidden(name: &str) -> bool {
    name == QUESTION_TOOL || name == EXIT_PLAN_TOOL
}

/// Starts a subagent. Older builds called it Task.
pub(super) fn is_subagent(name: &str) -> bool {
    matches!(name, "Agent" | "Task")
}

/// How Claude names a tool served by an MCP server.
pub fn mcp_tool_name(server: &str, tool: &str) -> String {
    format!("mcp__{server}__{tool}")
}

/// A tool call as the thread shows it.
pub struct ToolUse {
    pub role: ToolRole,
    /// The command for a shell call, otherwise the tool as words.
    pub title: String,
    pub detail: Option<String>,
    pub locations: Vec<String>,
    pub hidden: bool,
}

impl ToolUse {
    /// What a permission request names: the command a shell call would run, or the file a tool
    /// would touch.
    pub fn asks_about(&self) -> Option<String> {
        match &self.role {
            ToolRole::Command { command, .. } => (!command.is_empty()).then(|| command.clone()),
            _ => self.locations.first().cloned(),
        }
    }
}

pub fn tool_use(name: &str, input: &Value) -> ToolUse {
    let field = |key: &str| input.get(key).and_then(Value::as_str).map(str::to_string);
    let path = field("file_path").or_else(|| field("notebook_path"));
    let role = role(name, input);
    let (title, detail) = match &role {
        ToolRole::Command {
            command,
            description,
        } => (
            if command.is_empty() {
                display_name(name)
            } else {
                command.clone()
            },
            description.clone(),
        ),
        _ => (
            display_name(name),
            path.clone()
                .or_else(|| field("path"))
                .or_else(|| field("pattern"))
                .or_else(|| field("url"))
                .or_else(|| field("query"))
                .or_else(|| field("skill"))
                .or_else(|| field("description")),
        ),
    };
    ToolUse {
        role,
        title,
        detail,
        locations: path.into_iter().collect(),
        hidden: is_hidden(name),
    }
}

pub fn role(name: &str, input: &Value) -> ToolRole {
    let field = |key: &str| {
        input
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    match name {
        "Read" | "LS" | "NotebookRead" => ToolRole::Read,
        "Glob" | "Grep" => ToolRole::Search,
        "Edit" | "Write" | "NotebookEdit" => ToolRole::Edit {
            verb: EditVerb::Edited,
        },
        "Bash" | "BashOutput" => ToolRole::Command {
            command: field("command"),
            description: input
                .get("description")
                .and_then(Value::as_str)
                .map(str::to_string),
        },
        "WebFetch" => ToolRole::Fetch { url: field("url") },
        "WebSearch" => ToolRole::WebSearch {
            query: field("query"),
        },
        "Skill" => ToolRole::Skill {
            name: field("skill"),
        },
        name if is_subagent(name) => ToolRole::Subagent {
            description: input
                .get("description")
                .and_then(Value::as_str)
                .map(str::to_string),
        },
        "ToolSearch" => ToolRole::ToolLookup,
        ENTER_PLAN_TOOL => ToolRole::PlanMode,
        _ => match parse_mcp_name(name) {
            Some((server, action)) => ToolRole::Mcp { server, action },
            None => ToolRole::Other {
                name: humanize(name),
            },
        },
    }
}

/// A tool name as words: `GitHub: get PR` for an MCP tool, `Web fetch` for `WebFetch`.
pub fn display_name(name: &str) -> String {
    match parse_mcp_name(name) {
        Some((server, action)) => format!("{server}: {action}"),
        None => humanize(name),
    }
}

/// `mcp__<server>__<tool>` as words, without claude.ai's prefix on its own connectors, and
/// without the server's name repeated at the start of the tool's (`trelloReadCard` on the Trello
/// server reads as `Read card`).
fn parse_mcp_name(name: &str) -> Option<(String, String)> {
    let rest = name.strip_prefix("mcp__")?;
    let split = rest.get(1..)?.find("__")? + 1;
    let (raw_server, tool) = (&rest[..split], &rest[split + 2..]);
    if tool.is_empty() {
        return None;
    }
    let server = raw_server.strip_prefix("claude_ai_").unwrap_or(raw_server);
    Some((humanize(server), humanize(without_prefix(tool, server))))
}

fn without_prefix<'a>(name: &'a str, prefix: &str) -> &'a str {
    let starts = name
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix));
    if !starts {
        return name;
    }
    let rest = &name[prefix.len()..];
    let mut chars = rest.chars();
    let follows = match chars.next() {
        Some(first) if first.is_ascii_uppercase() => true,
        Some('_' | '-') => rest
            .trim_start_matches(['_', '-'])
            .starts_with(|c: char| c.is_ascii_alphabetic()),
        _ => false,
    };
    if follows {
        rest.trim_start_matches(['_', '-'])
    } else {
        name
    }
}

/// A camel, Pascal, snake or kebab case name as words (`getPRStatus` → `Get PR status`).
/// Anything that isn't a bare identifier, like a sentence or a path, is left alone.
pub fn humanize(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let identifier = chars.first().is_some_and(char::is_ascii_alphabetic)
        && chars
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-');
    if !identifier {
        return text.to_string();
    }
    let mut spaced = String::new();
    for (index, &c) in chars.iter().enumerate() {
        let previous = index.checked_sub(1).map(|at| chars[at]);
        let next = chars.get(index + 1);
        let lower_then_upper = previous
            .is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit())
            && c.is_ascii_uppercase();
        let acronym_then_word = previous.is_some_and(|p| p.is_ascii_uppercase())
            && c.is_ascii_uppercase()
            && next.is_some_and(char::is_ascii_lowercase);
        if lower_then_upper || acronym_then_word {
            spaced.push(' ');
        }
        spaced.push(c);
    }
    let words: Vec<String> = spaced
        .split([' ', '_', '-'])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let acronym = word.len() >= 2
                && word
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
            if acronym {
                word.to_string()
            } else {
                word.to_lowercase()
            }
        })
        .collect();
    let mut sentence = words.join(" ");
    if let Some(first) = sentence.get(..1) {
        sentence = first.to_uppercase() + &sentence[1..];
    }
    sentence
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_permission_names_the_command_or_the_file() {
        for (name, input, asks_about) in [
            (
                "Bash",
                json!({ "command": "ls", "description": "List" }),
                Some("ls"),
            ),
            ("Bash", json!({}), None),
            ("Edit", json!({ "file_path": "/a.rs" }), Some("/a.rs")),
            (
                "NotebookEdit",
                json!({ "notebook_path": "/n.ipynb" }),
                Some("/n.ipynb"),
            ),
            ("Grep", json!({ "pattern": "todo" }), None),
        ] {
            assert_eq!(
                tool_use(name, &input).asks_about().as_deref(),
                asks_about,
                "{name}"
            );
        }
    }

    #[test]
    fn names_read_as_words() {
        for (name, words) in [
            ("trelloReadCard", "Trello read card"),
            ("TodoWrite", "Todo write"),
            ("list_pull_requests", "List pull requests"),
            ("create-issue", "Create issue"),
            ("getPRStatus", "Get PR status"),
            ("URL", "URL"),
            ("Read", "Read"),
            ("Opened PR #12", "Opened PR #12"),
            ("src/foo_bar.ts", "src/foo_bar.ts"),
        ] {
            assert_eq!(humanize(name), words, "{name}");
        }
    }

    #[test]
    fn an_mcp_tool_is_named_by_its_server_and_action() {
        assert_eq!(
            display_name("mcp__claude_ai_Trello__trelloReadCard"),
            "Trello: Read card"
        );
        assert_eq!(display_name("WebFetch"), "Web fetch");
        assert_eq!(
            role("mcp__github__get_pr", &json!({})),
            ToolRole::Mcp {
                server: "Github".into(),
                action: "Get pr".into(),
            }
        );
    }

    #[test]
    fn each_tool_gets_its_role_from_its_name_and_input() {
        let cases = [
            ("Read", json!({}), ToolRole::Read),
            ("Grep", json!({}), ToolRole::Search),
            (
                "Write",
                json!({}),
                ToolRole::Edit {
                    verb: EditVerb::Edited,
                },
            ),
            (
                "Bash",
                json!({ "command": "npm run preview", "description": "Preview" }),
                ToolRole::Command {
                    command: "npm run preview".into(),
                    description: Some("Preview".into()),
                },
            ),
            (
                "WebFetch",
                json!({ "url": "https://a.dev" }),
                ToolRole::Fetch {
                    url: "https://a.dev".into(),
                },
            ),
            (
                "WebSearch",
                json!({ "query": "rust" }),
                ToolRole::WebSearch {
                    query: "rust".into(),
                },
            ),
            (
                "Skill",
                json!({ "skill": "pdf" }),
                ToolRole::Skill { name: "pdf".into() },
            ),
            (
                "Task",
                json!({ "description": "Find auth" }),
                ToolRole::Subagent {
                    description: Some("Find auth".into()),
                },
            ),
            ("ToolSearch", json!({}), ToolRole::ToolLookup),
            (
                "KillShell",
                json!({}),
                ToolRole::Other {
                    name: "Kill shell".into(),
                },
            ),
        ];
        for (name, input, expected) in cases {
            assert_eq!(role(name, &input), expected, "{name}");
        }
    }

    #[test]
    fn questions_and_plan_exits_are_hidden_steps() {
        assert!(tool_use(QUESTION_TOOL, &json!({})).hidden);
        assert!(tool_use(EXIT_PLAN_TOOL, &json!({})).hidden);
        assert!(!tool_use("Read", &json!({})).hidden);
    }
}
