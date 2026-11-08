//! A repo's setup and run commands: what's saved in Settings, what the repo commits in
//! `.cormux/config.json`, and what Cormux suggests for a repo it hasn't seen. The committed file
//! is read whenever a command is needed, so a teammate's change to it takes effect; what Settings
//! saves overrides it field by field, and a blank field follows the file.

use std::path::Path;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{Error, Result};
use crate::store::RustMigration;
use crate::store::types::{RepoRecord, WorkspaceRow};

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

pub fn effective_setup(stored: &str, repo_path: &Path) -> Result<String> {
    if !stored.trim().is_empty() {
        return Ok(stored.to_string());
    }
    Ok(read_committed_config(repo_path)?
        .map(|config| config.setup_commands)
        .unwrap_or_default())
}

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

/// The command every workspace of the repo runs unless it sets its own.
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

/// A repo's run command, for the webview: what Run starts in its workspaces unless they set
/// their own.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepoRunCommand {
    pub repo_id: String,
    pub command: String,
    pub source: RunCommandSource,
}

/// Each repo's run command, for those that have one. A committed config that can't be read
/// counts as none here; adding the repo and running its app report it.
pub fn repo_run_commands(repos: &[RepoRecord]) -> Vec<RepoRunCommand> {
    repos
        .iter()
        .filter_map(|repo| match repo_run_command(repo) {
            Ok(run) => run.map(|run| RepoRunCommand {
                repo_id: repo.id.clone(),
                command: run.command,
                source: run.source,
            }),
            Err(error) => {
                log::warn!("run command for {}: {error}", repo.id);
                None
            }
        })
        .collect()
}

pub fn workspace_run_command(own: Option<&str>, repo: &RepoRecord) -> Result<Option<RunCommand>> {
    match own_run_command(own) {
        Some(own) => Ok(Some(own)),
        None => repo_run_command(repo),
    }
}

fn own_run_command(own: Option<&str>) -> Option<RunCommand> {
    non_blank(own).map(|command| RunCommand {
        command,
        source: RunCommandSource::Workspace,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRunCommand {
    pub workspace_id: String,
    pub command: String,
    pub source: RunCommandSource,
}

pub fn workspace_run_commands(
    workspaces: &[WorkspaceRow],
    repos: &[RepoRunCommand],
) -> Vec<WorkspaceRunCommand> {
    workspaces
        .iter()
        .filter_map(|workspace| {
            let run = own_run_command(workspace.run_command.as_deref()).or_else(|| {
                repos
                    .iter()
                    .find(|repo| repo.repo_id == workspace.repo_id)
                    .map(|repo| RunCommand {
                        command: repo.command.clone(),
                        source: repo.source,
                    })
            })?;
            Some(WorkspaceRunCommand {
                workspace_id: workspace.id.clone(),
                command: run.command,
                source: run.source,
            })
        })
        .collect()
}

/// What to keep as a workspace's own command when the user types `typed`: nothing when it's
/// blank or just the repo's command again, so the workspace follows the repo's from then on.
pub fn own_run_command_to_save(typed: &str, repo: &RepoRecord) -> Result<Option<String>> {
    let typed = typed.trim();
    if typed.is_empty() {
        return Ok(None);
    }
    let repo_command = repo_run_command(repo)?.map(|run| run.command);
    Ok((repo_command.as_deref() != Some(typed)).then(|| typed.to_string()))
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

/// What a newly added repo saves in Settings: nothing for a field its committed config gives,
/// so the file stays in charge of it, and the suggestion for a field it doesn't.
pub fn initial_commands(
    committed: Option<&CommittedRepoConfig>,
    suggested: SuggestedRepoCommands,
) -> (String, Option<String>) {
    let committed_setup = committed.is_some_and(|c| !c.setup_commands.trim().is_empty());
    let committed_run = committed.is_some_and(|c| non_blank(c.run_command.as_deref()).is_some());
    let setup = if committed_setup {
        String::new()
    } else {
        suggested.setup_commands
    };
    let run = if committed_run {
        None
    } else {
        suggested.run_command
    };
    (setup, run)
}

/// A repo's committed setup, shown in Settings where its own is blank.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RepoCommittedSetup {
    pub repo_id: String,
    pub setup_commands: String,
}

/// The committed setup of each repo whose config has one. A config that can't be read counts as
/// none here; setting up a workspace reports it.
pub fn committed_setups(repos: &[RepoRecord]) -> Vec<RepoCommittedSetup> {
    repos
        .iter()
        .filter_map(|repo| {
            match read_committed_config(&crate::paths::expand_tilde(&repo.path)) {
                Ok(config) => config,
                Err(error) => {
                    log::warn!("setup commands for {}: {error}", repo.id);
                    None
                }
            }
            .filter(|config| !config.setup_commands.trim().is_empty())
            .map(|config| RepoCommittedSetup {
                repo_id: repo.id.clone(),
                setup_commands: config.setup_commands,
            })
        })
        .collect()
}

/// Earlier builds copied a repo's committed config into Settings when it was added, so later
/// changes to the file never took effect. A saved field that still matches the file is that
/// copy and goes back to following it; one the user changed stays as their override. Repos whose
/// file is missing or unreadable now are left as they are.
pub const MIGRATION: RustMigration = RustMigration {
    name: "030_follow_committed_repo_config",
    run: follow_committed_config,
};

fn follow_committed_config(conn: &Connection) -> Result<()> {
    let rows: Vec<(String, String, String, Option<String>)> = conn
        .prepare(
            "SELECT r.id, r.path, c.setup_commands, c.run_command
             FROM repos r JOIN repo_config c ON c.repo_id = r.id",
        )?
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    for (id, path, setup, run) in rows {
        let Ok(Some(config)) = read_committed_config(&crate::paths::expand_tilde(&path)) else {
            continue;
        };
        let copied = |saved: Option<&str>, committed: Option<&str>| matches!((non_blank(saved), non_blank(committed)), (Some(a), Some(b)) if a == b);
        if copied(Some(&setup), Some(&config.setup_commands)) {
            conn.execute(
                "UPDATE repo_config SET setup_commands = '' WHERE repo_id = ?1",
                [&id],
            )?;
        }
        if copied(run.as_deref(), config.run_command.as_deref()) {
            conn.execute(
                "UPDATE repo_config SET run_command = NULL WHERE repo_id = ?1",
                [&id],
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn repo(path: &Path, run_command: Option<&str>) -> RepoRecord {
        RepoRecord {
            path: path.to_string_lossy().to_string(),
            run_command: run_command.map(str::to_string),
            ..crate::store::fixtures::repo("app")
        }
    }

    fn commit_config(dir: &Path, json: &str) {
        fs::create_dir_all(dir.join(".cormux")).unwrap();
        fs::write(dir.join(".cormux/config.json"), json).unwrap();
    }

    #[test]
    fn each_workspace_runs_its_own_command_else_its_repos() {
        let repos = [RepoRunCommand {
            repo_id: "r1".into(),
            command: "pnpm dev".into(),
            source: RunCommandSource::Config,
        }];
        let workspace = |id: &str, repo: &str, own: Option<&str>| WorkspaceRow {
            repo_id: repo.into(),
            run_command: own.map(Into::into),
            ..crate::store::fixtures::workspace(id, crate::model::WorkspaceLifecycle::Idle)
        };
        let commands = workspace_run_commands(
            &[
                workspace("a", "r1", Some(" pnpm dev -p 4000 ")),
                workspace("b", "r1", Some("  ")),
                workspace("c", "r2", None),
            ],
            &repos,
        );
        assert_eq!(
            commands,
            [
                WorkspaceRunCommand {
                    workspace_id: "a".into(),
                    command: "pnpm dev -p 4000".into(),
                    source: RunCommandSource::Workspace,
                },
                WorkspaceRunCommand {
                    workspace_id: "b".into(),
                    command: "pnpm dev".into(),
                    source: RunCommandSource::Config,
                },
            ]
        );
    }

    #[test]
    fn typing_the_repos_command_again_keeps_no_override() {
        let dir = tempfile::tempdir().unwrap();
        let repo = repo(dir.path(), Some("pnpm dev"));
        assert_eq!(own_run_command_to_save(" pnpm dev ", &repo).unwrap(), None);
        assert_eq!(own_run_command_to_save("  ", &repo).unwrap(), None);
        assert_eq!(
            own_run_command_to_save(" pnpm dev -p 1 ", &repo)
                .unwrap()
                .as_deref(),
            Some("pnpm dev -p 1")
        );
    }

    #[test]
    fn a_workspace_override_wins_over_the_repo_command() {
        let dir = tempfile::tempdir().unwrap();
        let repo = repo(dir.path(), Some("pnpm dev"));
        let command = |own: Option<&str>| workspace_run_command(own, &repo).unwrap().unwrap();
        assert_eq!(command(None).command, "pnpm dev");
        assert_eq!(command(None).source, RunCommandSource::Repo);
        assert_eq!(
            command(Some(" pnpm dev --port 4000 ")),
            RunCommand {
                command: "pnpm dev --port 4000".into(),
                source: RunCommandSource::Workspace,
            }
        );
        assert_eq!(command(Some("  ")).command, "pnpm dev");
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
    fn a_new_repo_saves_suggestions_only_where_its_committed_config_says_nothing() {
        let suggested = || SuggestedRepoCommands {
            setup_commands: "pnpm install".into(),
            run_command: Some("pnpm dev".into()),
        };
        let setup_only = CommittedRepoConfig {
            setup_commands: "make setup".into(),
            run_command: Some("  ".into()),
        };
        assert_eq!(
            initial_commands(Some(&setup_only), suggested()),
            (String::new(), Some("pnpm dev".into()))
        );
        assert_eq!(
            initial_commands(None, suggested()),
            ("pnpm install".into(), Some("pnpm dev".into()))
        );
    }

    /// Run against a database as the migrations before it left one.
    #[test]
    fn rows_copied_from_the_committed_config_go_back_to_following_it() {
        let conn = Connection::open_in_memory().unwrap();
        crate::store::migrate::migrated_before(&conn, 30);
        let copied = tempfile::tempdir().unwrap();
        commit_config(
            copied.path(),
            r#"{"setupCommands":"pnpm install\n","runCommand":"pnpm dev"}"#,
        );
        let edited = tempfile::tempdir().unwrap();
        commit_config(
            edited.path(),
            r#"{"setupCommands":"pnpm install","runCommand":"pnpm dev"}"#,
        );
        let uncommitted = tempfile::tempdir().unwrap();
        for (id, dir, setup, run) in [
            ("copied", copied.path(), "pnpm install", Some("pnpm dev")),
            (
                "edited",
                edited.path(),
                "pnpm i && cp .env",
                Some("pnpm dev -p 4000"),
            ),
            (
                "uncommitted",
                uncommitted.path(),
                "pnpm install",
                Some("pnpm dev"),
            ),
        ] {
            conn.execute(
                "INSERT INTO repos (id, path, name) VALUES (?1, ?2, ?1)",
                rusqlite::params![id, dir.to_string_lossy()],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO repo_config (repo_id, setup_commands, run_command) VALUES (?1, ?2, ?3)",
                rusqlite::params![id, setup, run],
            )
            .unwrap();
        }

        crate::store::migrate::run(&conn, &[MIGRATION]).unwrap();

        let saved = |id: &str| -> (String, Option<String>) {
            conn.query_row(
                "SELECT setup_commands, run_command FROM repo_config WHERE repo_id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap()
        };
        assert_eq!(saved("copied"), (String::new(), None));
        assert_eq!(
            saved("edited"),
            ("pnpm i && cp .env".into(), Some("pnpm dev -p 4000".into())),
            "the user's own commands stay"
        );
        assert_eq!(
            saved("uncommitted"),
            ("pnpm install".into(), Some("pnpm dev".into()))
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
        assert_eq!(effective_setup("pnpm i", dir.path()).unwrap(), "pnpm i");
    }
}
