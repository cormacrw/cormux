//! Running the GitHub CLI. Every call gets the login-shell env and never waits on a prompt:
//! there's no terminal to answer it.

use std::path::Path;
use std::process::{Output, Stdio};
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::error::Error;
use crate::shell_env::ShellEnv;

const QUIET: [(&str, &str); 2] = [("GH_PROMPT_DISABLED", "1"), ("NO_COLOR", "1")];

pub const NOT_INSTALLED: &str = "GitHub CLI not found. Install gh";

/// Why `gh` didn't run at all.
#[derive(Debug)]
pub enum GhFailure {
    NotInstalled,
    Spawn(std::io::Error),
}

impl From<GhFailure> for Error {
    fn from(failure: GhFailure) -> Self {
        match failure {
            GhFailure::NotInstalled => Error::Github(format!("{NOT_INSTALLED}.")),
            GhFailure::Spawn(error) => Error::Github(format!("running gh: {error}")),
        }
    }
}

#[derive(Clone)]
pub struct Gh {
    env: Arc<RwLock<ShellEnv>>,
}

impl Gh {
    pub fn new(env: Arc<RwLock<ShellEnv>>) -> Self {
        Self { env }
    }

    /// Runs `gh args…`, whatever its exit status.
    pub async fn output(
        &self,
        cwd: Option<&Path>,
        args: &[&str],
        extra_env: &[(&str, &str)],
    ) -> std::result::Result<Output, GhFailure> {
        let mut command = self.env.read().await.command("gh");
        command.args(args);
        command.envs(QUIET);
        command.envs(extra_env.iter().copied());
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        command.stdin(Stdio::null());
        command.kill_on_drop(true);
        command.output().await.map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                GhFailure::NotInstalled
            } else {
                GhFailure::Spawn(error)
            }
        })
    }

    /// Runs `gh args…` and returns its trimmed stdout; a failed run is an error with its stderr.
    pub async fn stdout(&self, args: &[&str]) -> crate::error::Result<String> {
        let output = self.output(None, args, &[]).await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Github(format!(
                "gh {} failed: {}",
                args.first().copied().unwrap_or_default(),
                stderr.trim()
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_missing_gh_is_told_apart_from_a_failed_one() {
        // An env whose PATH has no `gh`.
        let env = ShellEnv::from_vars([("PATH".to_string(), "/nonexistent".to_string())]);
        let gh = Gh::new(Arc::new(RwLock::new(env)));
        let failure = gh.output(None, &["--version"], &[]).await.unwrap_err();
        assert!(matches!(failure, GhFailure::NotInstalled), "{failure:?}");
        let error = gh.stdout(&["--version"]).await.unwrap_err().to_string();
        assert!(error.contains(NOT_INSTALLED), "{error}");
    }
}
