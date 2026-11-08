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

    /// Replaces the command's environment with this one. Until the env has loaded it is
    /// empty, and the command keeps this process's.
    pub fn apply(&self, command: &mut impl EnvTarget) {
        if self.vars.is_empty() {
            return;
        }
        command.clear_env();
        for (key, value) in &self.vars {
            command.set_env(key, value);
        }
    }

    pub fn shell(&self) -> String {
        self.get("SHELL")
            .map(str::to_string)
            .or_else(|| std::env::var("SHELL").ok())
            .unwrap_or_else(|| "/bin/zsh".into())
    }

    pub fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        self.apply(&mut command);
        command
    }

    pub async fn load(&mut self) -> Result<()> {
        let shell = self.shell();
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

    /// Falls back to this process's environment so the app can still start.
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

    pub async fn run_with_stdin(
        &self,
        program: &str,
        args: &[&str],
        cwd: Option<&Path>,
        stdin: Option<&str>,
    ) -> Result<String> {
        let mut command = self.command(program);
        command.args(args);
        // A caller that times out or is cancelled drops this future; don't
        // leave the child running behind it.
        command.kill_on_drop(true);
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

/// A command [`ShellEnv::apply`] can set up: a plain child process or a PTY one.
pub trait EnvTarget {
    fn clear_env(&mut self);
    fn set_env(&mut self, key: &str, value: &str);
}

impl EnvTarget for Command {
    fn clear_env(&mut self) {
        self.env_clear();
    }

    fn set_env(&mut self, key: &str, value: &str) {
        self.env(key, value);
    }
}

impl EnvTarget for CommandBuilder {
    fn clear_env(&mut self) {
        self.env_clear();
    }

    fn set_env(&mut self, key: &str, value: &str) {
        self.env(key, value);
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

    #[test]
    fn parses_nul_delimited_env() {
        let vars = parse_env0(b"PATH=/opt/bin:/usr/bin\0HOME=/Users/dev\0").unwrap();
        assert_eq!(vars.get("PATH").unwrap(), "/opt/bin:/usr/bin");
        assert_eq!(vars.get("HOME").unwrap(), "/Users/dev");
    }

    /// A login shell that is a script: `body` stands in for `-l -i -c 'env -0'`.
    fn fake_shell(dir: &Path, body: &str) -> ShellEnv {
        use std::os::unix::fs::PermissionsExt;
        let shell = dir.join("login-shell");
        std::fs::write(&shell, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o755)).unwrap();
        ShellEnv::from_vars([("SHELL".to_string(), shell.display().to_string())])
    }

    #[tokio::test]
    async fn the_login_shells_env_replaces_the_apps() {
        let dir = tempfile::tempdir().unwrap();
        let mut env = fake_shell(dir.path(), r"printf 'PATH=/login/bin\0HOME=/Users/dev\0'");
        env.load().await.unwrap();
        assert_eq!(env.get("PATH"), Some("/login/bin"));
        assert_eq!(env.get("SHELL"), None, "nothing is kept from before");

        let mut command = env.command("/usr/bin/env");
        let output = command.output().await.unwrap();
        let seen = String::from_utf8(output.stdout).unwrap();
        assert!(seen.contains("HOME=/Users/dev"), "{seen}");
    }

    #[tokio::test]
    async fn a_broken_login_shell_falls_back_to_the_apps_env() {
        let dir = tempfile::tempdir().unwrap();
        let mut env = fake_shell(dir.path(), "exit 1");
        assert!(env.clone().load().await.is_err());
        env.load_or_inherit().await.unwrap();
        assert_eq!(env.get("PATH"), std::env::var("PATH").ok().as_deref());

        let mut silent = fake_shell(dir.path(), "true");
        assert!(silent.load().await.is_err(), "an empty env is a failure");
    }
}
