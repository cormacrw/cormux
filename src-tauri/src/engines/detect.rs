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
    let mut out = Vec::new();
    for kind in EngineKind::all() {
        out.push(detect_one(env, *kind).await);
    }
    Ok(out)
}

async fn detect_one(env: &ShellEnv, kind: EngineKind) -> EngineStatus {
    let Some(binary) = resolve_binary(env, kind.binaries()) else {
        return EngineStatus {
            kind,
            installed: false,
            binary: None,
            version: None,
            signed_in: None,
        };
    };

    let version = version_line(env, &binary).await;
    let signed_in = match kind {
        EngineKind::Claude => claude_signed_in(env, &binary).await,
        _ => None,
    };

    EngineStatus {
        kind,
        installed: true,
        binary: Some(binary.display().to_string()),
        version,
        signed_in,
    }
}

pub fn resolve_binary(env: &ShellEnv, names: &[&str]) -> Option<PathBuf> {
    let path = env
        .get("PATH")
        .map(ToString::to_string)
        .or_else(|| std::env::var("PATH").ok())?;
    for dir in path.split(':') {
        for name in names {
            let candidate = Path::new(dir).join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
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

async fn claude_signed_in(env: &ShellEnv, binary: &Path) -> Option<bool> {
    let mut command = Command::new(binary);
    env.apply(&mut command);
    command.args(["auth", "status"]).stdin(Stdio::null());
    let output = timeout(Duration::from_secs(3), command.output())
        .await
        .ok()?
        .ok()?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
    .to_lowercase();
    if text.contains("logged in") || text.contains("logged-in") {
        Some(true)
    } else if text.contains("not logged") || text.contains("unauthenticated") {
        Some(false)
    } else {
        None
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
        let status = detect_one(&env, EngineKind::Gemini).await;
        assert!(!status.installed);
        assert!(status.binary.is_none());
    }

    #[tokio::test]
    async fn finds_stub_binary_on_path() {
        let dir = tempfile::tempdir().unwrap();
        let stub = dir.path().join("gemini");
        std::fs::write(&stub, "#!/bin/sh\necho gemini 9.9.9\n").unwrap();
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
        let env = ShellEnv::from_vars([("PATH".into(), dir.path().display().to_string())]);
        let status = detect_one(&env, EngineKind::Gemini).await;
        assert!(status.installed);
        assert_eq!(status.version.as_deref(), Some("gemini 9.9.9"));
    }
}
