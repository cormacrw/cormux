use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::process::Command;
use tokio::time::timeout;

use super::events::EngineKind;
use crate::error::Result;
use crate::shell_env::ShellEnv;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub kind: EngineKind,
    pub installed: bool,
    pub binary: Option<String>,
    pub version: Option<String>,
    pub signed_in: Option<bool>,
}

pub async fn detect_engines(env: &ShellEnv) -> Result<Vec<EngineStatus>> {
    Ok(vec![detect_claude(env).await])
}

async fn detect_claude(env: &ShellEnv) -> EngineStatus {
    let kind = EngineKind::Claude;
    let Some(binary) = resolve_binary(env, CLAUDE_BINARY) else {
        return EngineStatus {
            kind,
            installed: false,
            binary: None,
            version: None,
            signed_in: None,
        };
    };

    let version = version_line(env, &binary).await;
    let signed_in = claude_signed_in(env, &binary).await;

    EngineStatus {
        kind,
        installed: true,
        binary: Some(binary.display().to_string()),
        version,
        signed_in,
    }
}

pub const CLAUDE_BINARY: &str = "claude";

/// The first `name` on the shell's PATH.
pub fn resolve_binary(env: &ShellEnv, name: &str) -> Option<PathBuf> {
    let path = env
        .get("PATH")
        .map(ToString::to_string)
        .or_else(|| std::env::var("PATH").ok())?;
    path.split(':')
        .map(|dir| Path::new(dir).join(name))
        .find(|candidate| candidate.is_file())
}

async fn version_line(env: &ShellEnv, binary: &Path) -> Option<String> {
    let mut command = Command::new(binary);
    env.apply(&mut command);
    command.arg("--version").stdin(Stdio::null());
    let output = timeout(Duration::from_secs(3), command.output())
        .await
        .ok()?
        .ok()?;
    let text = if output.status.success() {
        String::from_utf8_lossy(&output.stdout)
    } else {
        String::from_utf8_lossy(&output.stderr)
    };
    let line = text.lines().next()?.trim();
    if line.is_empty() {
        None
    } else {
        Some(line.to_string())
    }
}

/// Reads `claude auth status`, which prints JSON such as `{"loggedIn": true, ...}`. Unknown when
/// it can't be run or says something else.
async fn claude_signed_in(env: &ShellEnv, binary: &Path) -> Option<bool> {
    let mut command = Command::new(binary);
    env.apply(&mut command);
    command.args(["auth", "status"]).stdin(Stdio::null());
    let output = timeout(Duration::from_secs(3), command.output())
        .await
        .ok()?
        .ok()?;
    signed_in_from(&output.stdout)
}

fn signed_in_from(stdout: &[u8]) -> Option<bool> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct AuthStatus {
        logged_in: bool,
    }
    match serde_json::from_slice::<AuthStatus>(stdout) {
        Ok(status) => Some(status.logged_in),
        Err(error) => {
            log::warn!("reading `claude auth status`: {error}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[tokio::test]
    async fn missing_engine_is_not_installed() {
        let dir = tempfile::tempdir().unwrap();
        let env = ShellEnv::from_vars([("PATH".into(), dir.path().display().to_string())]);
        let status = detect_claude(&env).await;
        assert!(!status.installed);
        assert!(status.binary.is_none());
    }

    #[tokio::test]
    async fn finds_stub_binary_on_path() {
        let dir = tempfile::tempdir().unwrap();
        let stub = dir.path().join("claude");
        std::fs::write(&stub, "#!/bin/sh\necho claude 9.9.9\n").unwrap();
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
        let env = ShellEnv::from_vars([("PATH".into(), dir.path().display().to_string())]);
        let status = detect_claude(&env).await;
        assert!(status.installed);
        assert_eq!(status.version.as_deref(), Some("claude 9.9.9"));
    }

    #[tokio::test]
    async fn sign_in_is_read_from_the_cli_json() {
        for (logged_in, expected) in [("true", Some(true)), ("false", Some(false))] {
            let dir = tempfile::tempdir().unwrap();
            let stub = dir.path().join("claude");
            std::fs::write(
                &stub,
                format!(
                    "#!/bin/sh\nif [ \"$1\" = auth ]; then\n  printf '{{\\n  \"loggedIn\": {logged_in},\\n  \"authMethod\": \"claude.ai\"\\n}}\\n'\nelse\n  echo claude 9.9.9\nfi\n"
                ),
            )
            .unwrap();
            std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
            let env = ShellEnv::from_vars([("PATH".into(), dir.path().display().to_string())]);
            assert_eq!(detect_claude(&env).await.signed_in, expected);
        }
    }

    #[test]
    fn sign_in_is_unknown_when_the_cli_says_something_else() {
        assert_eq!(signed_in_from(b"Not logged in"), None);
        assert_eq!(signed_in_from(b"{}"), None);
    }
}
