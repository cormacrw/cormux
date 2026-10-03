use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;

use portable_pty::CommandBuilder;
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

    #[cfg(test)]
    pub fn from_vars(vars: impl IntoIterator<Item = (String, String)>) -> Self {
        Self {
            vars: vars.into_iter().collect(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(String::as_str)
    }

    pub fn vars(&self) -> &HashMap<String, String> {
        &self.vars
    }

    pub fn apply(&self, command: &mut Command) {
        if self.vars.is_empty() {
            return;
        }
        command.env_clear();
        for (key, value) in &self.vars {
            command.env(key, value);
        }
    }

    pub fn apply_pty(&self, command: &mut CommandBuilder) {
        if self.vars.is_empty() {
            return;
        }
        command.env_clear();
        for (key, value) in &self.vars {
            command.env(key, value);
        }
    }

    pub fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        self.apply(&mut command);
        command
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

    /// Prefer the login shell; if it fails, inherit this process's environment
    /// so the app can still start.
    pub async fn load_or_inherit(&mut self) -> Result<()> {
        match self.load().await {
            Ok(()) => Ok(()),
            Err(error) => {
                log::warn!("login shell env failed ({error}); inheriting process env");
                self.vars = std::env::vars().collect();
                if self.vars.is_empty() {
                    Err(error)
                } else {
                    Ok(())
                }
            }
        }
    }

    pub async fn reload(&mut self) -> Result<()> {
        self.load().await
    }

    pub async fn run(&self, program: &str, args: &[&str], cwd: Option<&Path>) -> Result<String> {
        self.run_with_stdin(program, args, cwd, None).await
    }

    pub async fn run_with_stdin(
        &self,
        program: &str,
        args: &[&str],
        cwd: Option<&Path>,
        stdin: Option<&str>,
    ) -> Result<String> {
        let mut command = self.command(program);
        command.args(args);
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        let output = if let Some(input) = stdin {
            command.stdin(Stdio::piped());
            command.stdout(Stdio::piped());
            command.stderr(Stdio::piped());
            let mut child = command
                .spawn()
                .map_err(|error| Error::ShellEnv(error.to_string()))?;
            if let Some(mut handle) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                handle
                    .write_all(input.as_bytes())
                    .await
                    .map_err(|error| Error::ShellEnv(error.to_string()))?;
            }
            child
                .wait_with_output()
                .await
                .map_err(|error| Error::ShellEnv(error.to_string()))?
        } else {
            command.stdin(Stdio::null());
            command
                .output()
                .await
                .map_err(|error| Error::ShellEnv(error.to_string()))?
        };
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

    #[tokio::test]
    async fn reload_replaces_cached_vars() {
        let mut env = ShellEnv::new();
        env.load().await.unwrap();
        let first = env.get("PATH").unwrap().to_string();
        env.reload().await.unwrap();
        assert_eq!(env.get("PATH").unwrap(), first);
    }
}
