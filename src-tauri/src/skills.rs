//! Claude Code skills the composer's `/` picker offers: the repo's `.claude/skills` first,
//! then the user's `~/.claude/skills`, then plugin skills, then the skills built into Claude
//! Code. Each skill on disk is a folder with a `SKILL.md` whose frontmatter names and
//! describes it. Plugin skills are namespaced by their plugin, as `plugin:skill`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::ipc::commands::expand_tilde;
use crate::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub name: String,
    pub description: String,
    /// `project` for the repo's own skills, `user` for `~/.claude/skills`, `plugin` for
    /// those from Claude Code plugins, `builtin` for those that ship with Claude Code.
    pub source: String,
}

/// Plugin namespace Claude Code gives the skills claude.ai syncs into `~/.claude/skills/synced`.
const SYNCED_PLUGIN: &str = "anthropic-skills";

/// Skills that ship inside the Claude Code CLI, so no folder holds them. Kept by hand; update
/// it as Claude Code adds or drops skills.
const BUILTIN: &[(&str, &str)] = &[
    (
        "claude-api",
        "Reference for the Claude API and Anthropic SDK",
    ),
    ("code-review", "Review the current diff or a PR for bugs"),
    (
        "fewer-permission-prompts",
        "Allowlist common read-only tool calls to cut permission prompts",
    ),
    ("init", "Write a CLAUDE.md documenting the codebase"),
    (
        "keybindings-help",
        "Customize Claude Code keyboard shortcuts",
    ),
    (
        "loop",
        "Run a prompt or slash command on a recurring interval",
    ),
    (
        "plugin-authoring",
        "Write a Claude Code plugin of function hooks",
    ),
    ("run", "Launch the app to see a change working"),
    ("schedule", "Create or run scheduled cloud agents"),
    (
        "security-review",
        "Security review of the pending changes on the branch",
    ),
    (
        "simplify",
        "Clean up the changed code for reuse, simplicity and efficiency",
    ),
    (
        "update-config",
        "Configure Claude Code via settings.json: hooks, permissions, env",
    ),
];

/// Skills available to a thread's agent, by the folder it runs in.
pub fn for_thread(state: &AppState, thread_id: &str) -> Result<Vec<Skill>> {
    let cwd = thread_cwd(state, thread_id)?;
    let home = std::env::var_os("HOME").map(PathBuf::from);
    Ok(list(&cwd, home.as_deref()))
}

fn thread_cwd(state: &AppState, thread_id: &str) -> Result<PathBuf> {
    if let Some(scratch) = state.store.scratch_for_thread(thread_id)? {
        let repo = state
            .store
            .snapshot()?
            .repos
            .into_iter()
            .find(|row| row.id == scratch.repo_id)
            .ok_or_else(|| Error::Workspace(format!("unknown repo {}", scratch.repo_id)))?;
        return Ok(expand_tilde(&repo.path));
    }
    let thread = state
        .store
        .thread_by_id(thread_id)?
        .ok_or_else(|| Error::Workspace(format!("unknown thread {thread_id}")))?;
    let workspace = state
        .store
        .workspace_by_id(&thread.workspace_id)?
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {}", thread.workspace_id)))?;
    Ok(PathBuf::from(workspace.worktree_path))
}

fn list(cwd: &Path, home: Option<&Path>) -> Vec<Skill> {
    let mut seen = HashSet::new();
    let mut skills = Vec::new();
    let roots = [
        Some((cwd.join(".claude/skills"), "project")),
        home.map(|home| (home.join(".claude/skills"), "user")),
    ];
    for (root, source) in roots.into_iter().flatten() {
        let mut found = read_root(&root, source);
        found.sort_by(|a, b| a.name.cmp(&b.name));
        for skill in found {
            // A project skill shadows a user skill of the same name, as in Claude Code.
            if seen.insert(skill.name.clone()) {
                skills.push(skill);
            }
        }
    }
    if let Some(home) = home {
        let mut found = plugin_skills(cwd, home);
        found.sort_by(|a, b| a.name.cmp(&b.name));
        for skill in found {
            if seen.insert(skill.name.clone()) {
                skills.push(skill);
            }
        }
    }
    for (name, description) in BUILTIN {
        if seen.insert(name.to_string()) {
            skills.push(Skill {
                name: name.to_string(),
                description: description.to_string(),
                source: "builtin".to_string(),
            });
        }
    }
    skills
}

fn read_root(root: &Path, source: &str) -> Vec<Skill> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let dir = entry.path();
            let text = std::fs::read_to_string(dir.join("SKILL.md")).ok()?;
            let folder = dir.file_name()?.to_str()?.to_string();
            let (name, description) = parse_frontmatter(&text);
            Some(Skill {
                name: name.unwrap_or(folder),
                description: description.unwrap_or_default(),
                source: source.to_string(),
            })
        })
        .collect()
}

/// Skills from claude.ai's synced plugin and from enabled, installed Claude Code plugins.
fn plugin_skills(cwd: &Path, home: &Path) -> Vec<Skill> {
    let mut skills = Vec::new();
    // Synced skills sit in the folder itself, or one level down in a per-account folder.
    let synced = home.join(".claude/skills/synced");
    let mut synced_roots = vec![synced.clone()];
    if let Ok(entries) = std::fs::read_dir(&synced) {
        synced_roots.extend(
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|dir| dir.is_dir() && !dir.join("SKILL.md").exists()),
        );
    }
    for root in synced_roots {
        skills.extend(namespaced(SYNCED_PLUGIN, read_root(&root, "plugin")));
    }
    for (plugin, install_path) in enabled_plugins(cwd, home) {
        skills.extend(namespaced(
            &plugin,
            read_root(&install_path.join("skills"), "plugin"),
        ));
    }
    skills
}

fn namespaced(plugin: &str, skills: Vec<Skill>) -> Vec<Skill> {
    skills
        .into_iter()
        .map(|skill| Skill {
            name: format!("{plugin}:{}", skill.name),
            ..skill
        })
        .collect()
}

/// Installed plugins (`name@marketplace`) that a user or project settings file enables,
/// with where each is installed.
fn enabled_plugins(cwd: &Path, home: &Path) -> Vec<(String, PathBuf)> {
    let read_json = |path: PathBuf| -> Option<serde_json::Value> {
        serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
    };
    let mut enabled = std::collections::HashMap::new();
    for settings in [
        home.join(".claude/settings.json"),
        cwd.join(".claude/settings.json"),
        cwd.join(".claude/settings.local.json"),
    ] {
        let Some(map) =
            read_json(settings).and_then(|json| json.get("enabledPlugins")?.as_object().cloned())
        else {
            continue;
        };
        // Later files override earlier ones, as in Claude Code.
        for (key, value) in map {
            enabled.insert(key, value.as_bool().unwrap_or(false));
        }
    }
    let Some(installed) = read_json(home.join(".claude/plugins/installed_plugins.json"))
        .and_then(|json| json.get("plugins")?.as_object().cloned())
    else {
        return Vec::new();
    };
    let mut plugins: Vec<(String, PathBuf)> = installed
        .into_iter()
        .filter(|(key, _)| enabled.get(key).copied().unwrap_or(false))
        .filter_map(|(key, installs)| {
            let path = installs.as_array()?.first()?.get("installPath")?.as_str()?;
            let name = key.split('@').next().unwrap_or(&key).to_string();
            Some((name, PathBuf::from(path)))
        })
        .collect();
    plugins.sort();
    plugins
}

/// `name` and `description` from a `---` block. Handles plain, quoted and folded
/// (`>` or `|`) values, which is all SKILL.md files use in practice.
fn parse_frontmatter(text: &str) -> (Option<String>, Option<String>) {
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("---") {
        return (None, None);
    }
    let block: Vec<&str> = lines.take_while(|line| line.trim() != "---").collect();
    let value = |key: &str| -> Option<String> {
        let prefix = format!("{key}:");
        let index = block.iter().position(|line| line.starts_with(&prefix))?;
        let raw = block[index][prefix.len()..].trim();
        let text = if raw.is_empty() || raw.starts_with('>') || raw.starts_with('|') {
            block[index + 1..]
                .iter()
                .take_while(|line| line.starts_with(' ') || line.starts_with('\t'))
                .map(|line| line.trim())
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            raw.trim_matches(|c| c == '"' || c == '\'').to_string()
        };
        Some(text).filter(|text| !text.is_empty())
    };
    (value("name"), value("description"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_plain_quoted_and_folded_values() {
        let text = "---\nname: deploy\ndescription: \"Ship it\"\n---\n# Deploy\n";
        assert_eq!(
            parse_frontmatter(text),
            (Some("deploy".into()), Some("Ship it".into()))
        );
        let folded = "---\nname: review\ndescription: >\n  Review the diff\n  for bugs.\n---\n";
        assert_eq!(
            parse_frontmatter(folded),
            (
                Some("review".into()),
                Some("Review the diff for bugs.".into())
            )
        );
        assert_eq!(parse_frontmatter("# No frontmatter"), (None, None));
    }

    #[test]
    fn lists_synced_and_enabled_plugin_skills_under_their_plugin() {
        let root = std::env::temp_dir().join(format!("cormux-skills-{}", uuid::Uuid::new_v4()));
        let home = root.join("home");
        let write = |path: PathBuf, body: &str| {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, body).unwrap();
        };
        let skill = |name: &str| format!("---\nname: {name}\ndescription: About {name}\n---\n");
        write(
            home.join(".claude/skills/synced/pdf/SKILL.md"),
            &skill("pdf"),
        );
        write(
            home.join(".claude/skills/synced/account-1/meals/SKILL.md"),
            &skill("meals"),
        );
        write(home.join(".claude/skills/synced/manifest.json"), "{}");
        let installs = root.join("plugins");
        write(
            installs.join("hooks/skills/rules/SKILL.md"),
            &skill("rules"),
        );
        write(
            installs.join("off/skills/hidden/SKILL.md"),
            &skill("hidden"),
        );
        write(
            home.join(".claude/plugins/installed_plugins.json"),
            &serde_json::json!({
                "version": 2,
                "plugins": {
                    "hooks@market": [{ "installPath": installs.join("hooks") }],
                    "off@market": [{ "installPath": installs.join("off") }],
                }
            })
            .to_string(),
        );
        write(
            home.join(".claude/settings.json"),
            r#"{"enabledPlugins":{"hooks@market":true,"off@market":true}}"#,
        );
        write(
            root.join("repo/.claude/settings.local.json"),
            r#"{"enabledPlugins":{"off@market":false}}"#,
        );

        let skills = list(&root.join("repo"), Some(&home));
        let _ = std::fs::remove_dir_all(&root);
        let plugin: Vec<_> = skills
            .iter()
            .filter(|skill| skill.source == "plugin")
            .map(|skill| skill.name.as_str())
            .collect();
        assert_eq!(
            plugin,
            [
                "anthropic-skills:meals",
                "anthropic-skills:pdf",
                "hooks:rules"
            ]
        );
        // The synced folder itself isn't a skill.
        assert!(!skills.iter().any(|skill| skill.name == "synced"));
    }

    #[test]
    fn project_skills_shadow_user_skills_which_shadow_builtins() {
        let root = std::env::temp_dir().join(format!("cormux-skills-{}", uuid::Uuid::new_v4()));
        let write = |dir: &str, body: &str| {
            let path = root.join(dir);
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(path.join("SKILL.md"), body).unwrap();
        };
        write(
            "repo/.claude/skills/deploy",
            "---\nname: deploy\ndescription: Repo deploy\n---\n",
        );
        write(
            "home/.claude/skills/deploy",
            "---\nname: deploy\ndescription: User deploy\n---\n",
        );
        write("home/.claude/skills/notes", "no frontmatter");

        write(
            "home/.claude/skills/simplify",
            "---\nname: simplify\ndescription: My simplify\n---\n",
        );

        let skills = list(&root.join("repo"), Some(&root.join("home")));
        let _ = std::fs::remove_dir_all(&root);
        let simplify = skills
            .iter()
            .find(|skill| skill.name == "simplify")
            .unwrap();
        assert_eq!(simplify.source, "user");
        assert!(
            skills
                .iter()
                .any(|skill| skill.name == "init" && skill.source == "builtin")
        );
        let on_disk: Vec<_> = skills
            .into_iter()
            .filter(|skill| skill.source != "builtin" && skill.name != "simplify")
            .collect();
        assert_eq!(
            on_disk,
            vec![
                Skill {
                    name: "deploy".into(),
                    description: "Repo deploy".into(),
                    source: "project".into(),
                },
                Skill {
                    name: "notes".into(),
                    description: String::new(),
                    source: "user".into(),
                },
            ]
        );
    }
}
