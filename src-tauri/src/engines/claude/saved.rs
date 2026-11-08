//! Tool calls and permission requests as builds before typed roles saved them: with Claude's
//! tool name (`name`, or only the title on the oldest rows) and a coarse kind. Used once, to
//! rewrite those rows the way the timeline drew them.

use super::tools::{display_name, humanize, is_hidden, is_subagent, role};
use crate::engines::events::{EditVerb, ToolRole};

/// A saved tool call's fields that carried its meaning.
pub struct SavedCall<'a> {
    pub name: Option<&'a str>,
    pub title: &'a str,
    /// `read`, `edit`, `delete`, `move`, `search`, `execute`, `fetch` or `other`.
    pub kind: &'a str,
    pub detail: Option<&'a str>,
}

pub struct RestoredCall {
    pub role: ToolRole,
    pub title: String,
    pub hidden: bool,
}

pub fn restore_call(saved: &SavedCall) -> RestoredCall {
    let tool = saved.name.unwrap_or(saved.title);
    let hidden = is_hidden(tool);
    let detail = saved.detail.unwrap_or_default().to_string();
    let (role, title) = match saved.kind {
        "edit" | "delete" | "move" => {
            let verb = if saved.kind == "delete" {
                EditVerb::Deleted
            } else if saved.title.to_lowercase().contains("creat") {
                EditVerb::Created
            } else {
                EditVerb::Edited
            };
            (ToolRole::Edit { verb }, saved.title.to_string())
        }
        // A shell call's title was its command, or the tool's name when it had none.
        "execute" if !is_subagent(tool) => {
            let command = saved.title.trim();
            let command = if saved.name == Some(command) {
                ""
            } else {
                command
            };
            (
                ToolRole::Command {
                    command: command.to_string(),
                    description: saved.detail.map(str::to_string),
                },
                saved.title.to_string(),
            )
        }
        _ => {
            let input = serde_json::json!({
                "url": detail,
                "query": detail,
                "skill": detail,
                "description": saved.detail,
            });
            let role = match role(tool, &input) {
                ToolRole::Read | ToolRole::Search | ToolRole::Other { .. } => match saved.kind {
                    "read" => ToolRole::Read,
                    "search" => ToolRole::Search,
                    _ => ToolRole::Other {
                        name: humanize(tool),
                    },
                },
                // Its kind said what it did; an edit or command is handled above.
                ToolRole::Edit { .. } | ToolRole::Command { .. } => ToolRole::Other {
                    name: humanize(tool),
                },
                named => named,
            };
            (role, humanize(saved.title))
        }
    };
    RestoredCall {
        role,
        title,
        hidden,
    }
}

/// A saved permission request's tool, as words and by what it does.
pub fn restore_permission_tool(
    tool_name: &str,
    kind: &str,
    detail: Option<&str>,
) -> (String, ToolRole) {
    let restored = restore_call(&SavedCall {
        name: Some(tool_name),
        title: detail.filter(|_| kind == "execute").unwrap_or(tool_name),
        kind,
        detail: None,
    });
    (display_name(tool_name), restored.role)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn restored(name: Option<&str>, title: &str, kind: &str, detail: Option<&str>) -> RestoredCall {
        restore_call(&SavedCall {
            name,
            title,
            kind,
            detail,
        })
    }

    #[test]
    fn a_saved_call_gets_the_role_its_name_and_kind_meant() {
        let shell = restored(Some("Bash"), "npm test", "execute", Some("Run tests"));
        assert_eq!(
            shell.role,
            ToolRole::Command {
                command: "npm test".into(),
                description: Some("Run tests".into()),
            }
        );
        let nameless = restored(Some("Bash"), "Bash", "execute", None);
        assert!(matches!(nameless.role, ToolRole::Command { command, .. } if command.is_empty()));

        let agent = restored(Some("Task"), "Task", "other", Some("Find auth"));
        assert_eq!(
            agent.role,
            ToolRole::Subagent {
                description: Some("Find auth".into()),
            }
        );
        let fetch = restored(Some("WebFetch"), "WebFetch", "fetch", Some("https://a.dev"));
        assert_eq!(
            fetch.role,
            ToolRole::Fetch {
                url: "https://a.dev".into(),
            }
        );
        let mcp = restored(None, "mcp__claude_ai_Trello__trelloReadCard", "other", None);
        assert_eq!(
            mcp.role,
            ToolRole::Mcp {
                server: "Trello".into(),
                action: "Read card".into(),
            }
        );
        let read = restored(
            Some("NotebookRead"),
            "NotebookRead",
            "read",
            Some("a.ipynb"),
        );
        assert_eq!(
            (read.role, read.title.as_str()),
            (ToolRole::Read, "Notebook read")
        );
        let created = restored(None, "Create file", "edit", None);
        assert_eq!(
            created.role,
            ToolRole::Edit {
                verb: EditVerb::Created,
            }
        );
        assert!(restored(Some("AskUserQuestion"), "AskUserQuestion", "other", None).hidden);
    }

    #[test]
    fn a_saved_permission_names_its_tool_in_words() {
        let (tool, role) = restore_permission_tool("mcp__github__get_pr", "other", None);
        assert_eq!(tool, "Github: Get pr");
        assert!(matches!(role, ToolRole::Mcp { .. }));
        let (tool, role) = restore_permission_tool("Bash", "execute", Some("ls"));
        assert_eq!(tool, "Bash");
        assert!(matches!(role, ToolRole::Command { command, .. } if command == "ls"));
    }
}
