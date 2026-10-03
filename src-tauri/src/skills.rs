//! Claude Code skills the composer's `/` picker offers: the repo's `.claude/skills` first,
//! then the user's `~/.claude/skills`. Each skill is a folder with a `SKILL.md` whose
//! frontmatter names and describes it.

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
    /// `project` for the repo's own skills, `user` for `~/.claude/skills`.
    pub source: String,
}

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
    fn project_skills_come_first_and_shadow_user_skills() {
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

        let skills = list(&root.join("repo"), Some(&root.join("home")));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(
            skills,
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
