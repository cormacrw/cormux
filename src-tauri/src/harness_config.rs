use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Error, Result};

const CONFIG_REL: &str = ".harness/config.json";

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommittedRepoConfig {
    #[serde(default)]
    pub setup_commands: String,
    pub run_command: Option<String>,
}

pub fn read_committed_config(repo_path: &Path) -> Option<CommittedRepoConfig> {
    let file = repo_path.join(CONFIG_REL);
    let text = std::fs::read_to_string(&file).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn effective_setup(stored: &str, repo_path: &Path) -> String {
    if !stored.trim().is_empty() {
        return stored.to_string();
    }
    read_committed_config(repo_path)
        .map(|config| config.setup_commands)
        .unwrap_or_default()
}

pub fn effective_run_command(stored: &Option<String>, repo_path: &Path) -> Option<String> {
    if let Some(value) = stored.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        return Some(value.to_string());
    }
    read_committed_config(repo_path).and_then(|config| {
        config
            .run_command
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
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

pub fn path_for_display(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        if canonical.starts_with(&home) {
            let rest = canonical
                .strip_prefix(&home)
                .unwrap_or(&canonical)
                .to_string_lossy()
                .trim_start_matches('/')
                .to_string();
            return format!("~/{rest}");
        }
    }
    canonical.to_string_lossy().to_string()
}

pub fn validate_add_path(path: &str) -> Result<(String, String)> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(Error::Workspace("Enter the path to a folder".into()));
    }
    let normalized = trimmed.trim_end_matches('/').to_string();
    let segment = normalized.rsplit('/').next().unwrap_or("").trim();
    if !normalized.starts_with("~/") && !normalized.starts_with('/') {
        return Err(Error::Workspace(
            "Use a full path, like ~/code/my-project".into(),
        ));
    }
    if segment.is_empty() || segment == "~" {
        return Err(Error::Workspace(
            "Use a full path, like ~/code/my-project".into(),
        ));
    }
    let id = segment.to_string();
    Ok((normalized, id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn validate_path_rules() {
        assert!(validate_add_path("").is_err());
        assert!(validate_add_path("relative/path").is_err());
        let (path, id) = validate_add_path("~/code/my-app/").unwrap();
        assert_eq!(path, "~/code/my-app");
        assert_eq!(id, "my-app");
    }

    #[test]
    fn reads_committed_config() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".harness")).unwrap();
        fs::write(
            dir.path().join(".harness/config.json"),
            r#"{"setupCommands":"pnpm install","runCommand":"pnpm dev"}"#,
        )
        .unwrap();
        let config = read_committed_config(dir.path()).expect("config");
        assert_eq!(config.setup_commands, "pnpm install");
        assert_eq!(config.run_command.as_deref(), Some("pnpm dev"));
    }

    #[test]
    fn effective_prefers_local_override() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".harness")).unwrap();
        fs::write(
            dir.path().join(".harness/config.json"),
            r#"{"setupCommands":"from file","runCommand":"file run"}"#,
        )
        .unwrap();
        assert_eq!(effective_setup("local setup", dir.path()), "local setup");
        assert_eq!(
            effective_run_command(&Some("local run".into()), dir.path()).as_deref(),
            Some("local run")
        );
        assert_eq!(effective_setup("", dir.path()), "from file");
        assert_eq!(
            effective_run_command(&None, dir.path()).as_deref(),
            Some("file run")
        );
    }
}
