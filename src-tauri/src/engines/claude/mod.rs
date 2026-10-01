//! Spike 1 (COR-42): Claude Code stream-JSON control protocol.
//!
//! Findings against Claude Code 2.1.x:
//! - `-p` is required; `--input-format stream-json` is print-mode only.
//! - `--permission-prompt-tool stdio` is still what the official SDK passes
//!   (it is not listed in `--help`). Pair it with `--permission-prompts host`.
//! - `--await-initialize` plus a host `control_request`/`initialize` is required
//!   before the CLI treats us as a permission surface. Without that handshake,
//!   asks become `system`/`permission_denied` instead of `can_use_tool`.
//! - With stdin held open, `system`/`init` often arrives only after the first
//!   user message, not during the initialize handshake.
//! - Interrupt is a host `control_request` with `subtype: interrupt`.
//! - Resume is a new process with `--resume <session_id>` from `system`/`init`.

pub mod map;
mod protocol;
mod session;

pub use protocol::{CanUseTool, Event, PermissionDecision};
pub use session::{ClaudeSession, SpawnOptions};

/// Argument vector for a streaming Claude Code session (binary not included).
pub fn claude_argv(resume: Option<&str>, model: Option<&str>) -> Vec<String> {
    let mut args = vec![
        "-p".into(),
        "--verbose".into(),
        "--input-format".into(),
        "stream-json".into(),
        "--output-format".into(),
        "stream-json".into(),
        "--permission-prompt-tool".into(),
        "stdio".into(),
        "--permission-prompts".into(),
        "host".into(),
        "--await-initialize".into(),
    ];
    if let Some(id) = resume {
        args.push("--resume".into());
        args.push(id.into());
    }
    if let Some(model) = model {
        args.push("--model".into());
        args.push(model.into());
    }
    args
}

#[cfg(test)]
mod tests {
    use super::claude_argv;

    #[test]
    fn argv_includes_print_and_stdio_permission_tool() {
        let args = claude_argv(None, None);
        assert!(args.contains(&"-p".into()));
        assert!(
            args.windows(2)
                .any(|w| w == ["--permission-prompt-tool", "stdio"])
        );
        assert!(args.contains(&"--await-initialize".into()));
    }

    #[test]
    fn argv_resume_appends_session_id() {
        let args = claude_argv(Some("sess-1"), None);
        assert!(args.windows(2).any(|w| w == ["--resume", "sess-1"]));
        assert!(!args.contains(&"--model".into()));
    }

    #[test]
    fn argv_model_appends_flag() {
        let args = claude_argv(None, Some("opus"));
        assert!(args.windows(2).any(|w| w == ["--model", "opus"]));
    }
}
