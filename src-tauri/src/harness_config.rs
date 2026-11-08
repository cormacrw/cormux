//! A repo's setup and run commands: what's saved in Settings, what the repo commits in
//! `.cormux/config.json`, and what Cormux suggests for a repo it hasn't seen.

use std::path::Path;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{Error, Result};
use crate::store::Store;
use crate::store::types::RepoRecord;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommittedRepoConfig {
    #[serde(default)]
    pub setup_commands: String,
    pub run_command: Option<String>,
}

/// The repo's committed config; `None` when it has none. A file that's there but can't be
/// read or parsed is an error, so a typo in it isn't mistaken for "no config".
pub fn read_committed_config(repo_path: &Path) -> Result<Option<CommittedRepoConfig>> {
    let file = crate::paths::repo_config(repo_path);
    let text = match std::fs::read_to_string(&file) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(config_error(&file, error)),
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|error| config_error(&file, error))
}

fn config_error(file: &Path, error: impl std::fmt::Display) -> Error {
    Error::Workspace(format!(
        "{} can't be used: {error}",
        crate::paths::for_display(file)
    ))
}

/// The setup commands a worktree gets: the repo's saved ones, else its committed ones.
pub fn effective_setup(stored: &str, repo_path: &Path) -> Result<String> {
    if !stored.trim().is_empty() {
        return Ok(stored.to_string());
    }
    Ok(read_committed_config(repo_path)?
        .map(|config| config.setup_commands)
        .unwrap_or_default())
}

/// Where a run command comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum RunCommandSource {
    /// The workspace's own, set in its Output tab.
    Workspace,
    /// The repo's, saved in Settings.
    Repo,
    /// The repo's committed `.cormux/config.json`.
    Config,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RunCommand {
    pub command: String,
    pub source: RunCommandSource,
}

fn non_blank(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn workspace_run_command_key(workspace_id: &str) -> String {
    format!("workspace:{workspace_id}/runCommand")
}

/// The command every workspace of the repo runs unless it sets its own: the saved one, else
/// the committed one.
pub fn repo_run_command(repo: &RepoRecord) -> Result<Option<RunCommand>> {
    if let Some(command) = non_blank(repo.run_command.as_deref()) {
        return Ok(Some(RunCommand {
            command,
            source: RunCommandSource::Repo,
        }));
    }
    let committed = read_committed_config(&crate::paths::expand_tilde(&repo.path))?;
    Ok(committed
        .and_then(|config| non_blank(config.run_command.as_deref()))
        .map(|command| RunCommand {
            command,
            source: RunCommandSource::Config,
        }))
}

/// The command Run starts for a workspace: its own when set, else the repo's.
pub fn workspace_run_command(
    store: &Store,
    workspace_id: &str,
    repo: &RepoRecord,
) -> Result<Option<RunCommand>> {
    let own = store.get_setting(&workspace_run_command_key(workspace_id))?;
    if let Some(command) = non_blank(own.as_deref()) {
        return Ok(Some(RunCommand {
            command,
            source: RunCommandSource::Workspace,
        }));
    }
    repo_run_command(repo)
}

#[derive(Debug, Clone, Default)]
pub struct SuggestedRepoCommands {
    pub setup_commands: String,
    pub run_command: Option<String>,
}

pub fn suggest_commands(repo_path: &Path) -> SuggestedRepoCommands {
    let mut setup_lines: Vec<String> = Vec::new();
    let mut run_command = None;

    let has_pnpm = repo_path.join("pnpm-lock.yaml").is_file();
    let has_yarn = repo_path.join("yarn.lock").is_file();
    let has_npm = repo_path.join("package-lock.json").is_file();
    let package_json = repo_path.join("package.json");

    if package_json.is_file() {
        if has_pnpm {
            setup_lines.push("pnpm install --frozen-lockfile".into());
        } else if has_yarn {
            setup_lines.push("yarn install --frozen-lockfile".into());
        } else if has_npm {
            setup_lines.push("npm ci".into());
        } else {
            setup_lines.push("pnpm install".into());
        }

        if let Ok(text) = std::fs::read_to_string(&package_json)
            && let Ok(json) = serde_json::from_str::<serde_json::Value>(&text)
            && let Some(scripts) = json.get("scripts").and_then(|v| v.as_object())
        {
            for key in ["dev", "start", "serve"] {
                if scripts.contains_key(key) {
                    let pm = if has_pnpm {
                        "pnpm"
                    } else if has_yarn {
                        "yarn"
                    } else {
                        "npm run"
                    };
                    run_command = Some(format!("{pm} {key}"));
                    break;
                }
            }
        }
    }

    if repo_path.join("pyproject.toml").is_file() {
        setup_lines.push("uv sync".into());
        if run_command.is_none() {
            run_command = Some("uv run python -m app".into());
        }
    }

    SuggestedRepoCommands {
        setup_commands: setup_lines.join("\n"),
        run_command,
    }
}

pub fn merge_initial_config(
    committed: Option<CommittedRepoConfig>,
    suggested: SuggestedRepoCommands,
) -> (String, Option<String>) {
    let setup = committed
        .as_ref()
        .map(|c| c.setup_commands.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| suggested.setup_commands);

    let run = committed
        .and_then(|c| c.run_command.filter(|s| !s.trim().is_empty()))
        .or(suggested.run_command);

    (setup, run)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn repo(path: &Path, run_command: Option<&str>) -> RepoRecord {
        RepoRecord {
            id: "app".into(),
            path: path.to_string_lossy().to_string(),
            name: "app".into(),
            default_branch: Some("main".into()),
            setup_commands: String::new(),
            run_command: run_command.map(str::to_string),
            single_instance: false,
        }
    }

    fn commit_config(dir: &Path, json: &str) {
        fs::create_dir_all(dir.join(".cormux")).unwrap();
        fs::write(dir.join(".cormux/config.json"), json).unwrap();
    }

    #[test]
    fn a_workspace_override_wins_over_the_repo_command() {
        let store = Store::in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let repo = repo(dir.path(), Some("pnpm dev"));
        let command = |id: &str| workspace_run_command(&store, id, &repo).unwrap().unwrap();
        assert_eq!(command("w1").command, "pnpm dev");
        assert_eq!(command("w1").source, RunCommandSource::Repo);
        store
            .set_setting("workspace:w1/runCommand", " pnpm dev --port 4000 ")
            .unwrap();
        assert_eq!(
            command("w1"),
            RunCommand {
                command: "pnpm dev --port 4000".into(),
                source: RunCommandSource::Workspace,
            }
        );
        assert_eq!(command("w2").command, "pnpm dev");
        // Clearing it goes back to the repo's.
        store.set_setting("workspace:w1/runCommand", "").unwrap();
        assert_eq!(command("w1").command, "pnpm dev");
    }

    #[test]
    fn the_committed_config_fills_in_what_settings_leave_blank() {
        let dir = tempfile::tempdir().unwrap();
        commit_config(
            dir.path(),
            r#"{"setupCommands":"from file","runCommand":"file run"}"#,
        );
        assert_eq!(
            effective_setup("local setup", dir.path()).unwrap(),
            "local setup"
        );
        assert_eq!(effective_setup("", dir.path()).unwrap(), "from file");
        assert_eq!(
            repo_run_command(&repo(dir.path(), Some("local run")))
                .unwrap()
                .unwrap()
                .command,
            "local run"
        );
        assert_eq!(
            repo_run_command(&repo(dir.path(), None)).unwrap(),
            Some(RunCommand {
                command: "file run".into(),
                source: RunCommandSource::Config,
            })
        );
    }

    #[test]
    fn a_broken_committed_config_is_an_error_not_an_absent_one() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_committed_config(dir.path()).unwrap().is_none());
        assert_eq!(repo_run_command(&repo(dir.path(), None)).unwrap(), None);

        commit_config(dir.path(), r#"{"setupCommands": "pnpm install",}"#);
        let error = read_committed_config(dir.path()).unwrap_err().to_string();
        assert!(
            error.contains(".cormux/config.json can't be used"),
            "{error}"
        );
        assert!(effective_setup("", dir.path()).is_err());
        // A saved command doesn't need the file.
        assert_eq!(effective_setup("pnpm i", dir.path()).unwrap(), "pnpm i");
    }
}
