use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;

use crate::error::{Error, Result};

/// Cached login-shell environment (`$SHELL -l -i -c 'env -0'`).
/// GUI apps on macOS do not inherit PATH, so every child process uses this.
#[derive(Debug, Clone, Default)]
pub struct ShellEnv {
    vars: HashMap<String, String>,
}

impl ShellEnv {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(String::as_str)
    }

    pub fn vars(&self) -> &HashMap<String, String> {
        &self.vars
    }

    pub fn apply(&self, command: &mut Command) {
        command.env_clear();
        for (key, value) in &self.vars {
            command.env(key, value);
        }
    }

    pub async fn load(&mut self) -> Result<()> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
        let output = Command::new(&shell)
            .args(["-l", "-i", "-c", "env -0"])
            .stdin(Stdio::null())
            .output()
            .await
            .map_err(|error| Error::ShellEnv(error.to_string()))?;
        if !output.status.success() {
            return Err(Error::ShellEnv(format!(
                "{shell} -l -i failed with {}",
                output.status
            )));
        }
        self.vars = parse_env0(&output.stdout)?;
        if self.vars.is_empty() {
            return Err(Error::ShellEnv("login shell returned an empty env".into()));
        }
        Ok(())
    }

    pub async fn reload(&mut self) -> Result<()> {
        self.load().await
    }

    pub async fn run(&self, program: &str, args: &[&str], cwd: Option<&Path>) -> Result<String> {
        let mut command = Command::new(program);
        command.args(args).stdin(Stdio::null());
        self.apply(&mut command);
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        let output = command
            .output()
            .await
            .map_err(|error| Error::ShellEnv(error.to_string()))?;
        if !output.status.success() {
            return Err(Error::ShellEnv(format!(
                "{program} failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

pub fn parse_env0(bytes: &[u8]) -> Result<HashMap<String, String>> {
    let mut vars = HashMap::new();
    for entry in bytes.split(|b| *b == 0) {
        if entry.is_empty() {
            continue;
        }
        let text =
            std::str::from_utf8(entry).map_err(|error| Error::ShellEnv(error.to_string()))?;
        let Some((key, value)) = text.split_once('=') else {
            continue;
        };
        vars.insert(key.to_string(), value.to_string());
    }
    Ok(vars)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_nul_delimited_env() {
        let vars = parse_env0(b"PATH=/opt/bin:/usr/bin\0HOME=/Users/dev\0").unwrap();
        assert_eq!(vars.get("PATH").unwrap(), "/opt/bin:/usr/bin");
        assert_eq!(vars.get("HOME").unwrap(), "/Users/dev");
    }

    #[tokio::test]
    async fn login_shell_recovers_tools_hidden_from_gui_path() {
        let mut env = ShellEnv::new();
        env.load().await.unwrap();
        let path = env.get("PATH").expect("PATH");
        assert!(path.contains('/'), "login PATH should be populated");

        let claude = env.run("claude", &["--version"], None).await;
        let pnpm = env.run("pnpm", &["--version"], None).await;
        assert!(
            claude.is_ok() || pnpm.is_ok(),
            "login env should resolve claude or pnpm; claude={claude:?} pnpm={pnpm:?}"
        );

        if pnpm.is_ok() {
            let dir = std::env::temp_dir().join("cormux-cor-44-worktree");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("package.json"),
                r#"{"name":"cor-44-spike","private":true}"#,
            )
            .unwrap();
            let _ = env
                .run("pnpm", &["install", "--ignore-scripts"], Some(&dir))
                .await;
            let _ = PathBuf::from(&dir);
        }
    }
}
