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
//! - An `sdk` server in `--mcp-config`, also named as `sdkMcpServers` in `initialize`, is
//!   answered by the host: each JSON-RPC message arrives as a `control_request` with
//!   `subtype: mcp_message`, and every one needs a `control_response`.
//! - A turn's `result` lands while background subagents run. `system`/`background_tasks_changed`
//!   lists what is live, and each agent that finishes starts a turn of its own with a new
//!   `system`/`init`. One stopped with `stop_task` starts none. Interrupt doesn't reach them.

pub mod map;
mod protocol;
mod session;
pub mod turn_gate;

pub use protocol::{CanUseTool, Event, PermissionDecision};
pub use session::{ClaudeSession, SpawnOptions};

/// Argument vector for a streaming Claude Code session (binary not included).
pub fn claude_argv(resume: Option<&str>, model: Option<&str>, orchestrate: bool) -> Vec<String> {
    let mut brief = format!(
        "{}\n\n{}",
        crate::engines::BRANCH_INSTRUCTION,
        crate::engines::MEDIA_INSTRUCTION
    );
    if orchestrate {
        brief.push_str("\n\n");
        brief.push_str(crate::mcp::ORCHESTRATION_INSTRUCTION);
    }
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
        "--append-system-prompt".into(),
        brief,
    ];
    if orchestrate {
        args.push("--mcp-config".into());
        args.push(crate::mcp::sdk_mcp_config());
        args.push("--allowedTools".into());
        args.push(crate::mcp::read_only_tools().join(","));
    }
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
        let args = claude_argv(None, None, false);
        assert!(args.contains(&"-p".into()));
        assert!(
            args.windows(2)
                .any(|w| w == ["--permission-prompt-tool", "stdio"])
        );
        assert!(args.contains(&"--await-initialize".into()));
    }

    #[test]
    fn argv_tells_claude_the_thread_renders_media() {
        let args = claude_argv(None, None, false);
        let at = args
            .iter()
            .position(|arg| arg == "--append-system-prompt")
            .expect("system prompt flag");
        assert!(args[at + 1].contains(crate::engines::BRANCH_INSTRUCTION));
        assert!(args[at + 1].contains(crate::engines::MEDIA_INSTRUCTION));
    }

    #[test]
    fn orchestrating_threads_get_the_cormux_server_and_its_brief() {
        let args = claude_argv(None, None, true);
        let at = args.iter().position(|arg| arg == "--mcp-config").unwrap();
        assert!(args[at + 1].contains(r#""type":"sdk""#));
        let at = args.iter().position(|arg| arg == "--allowedTools").unwrap();
        assert!(args[at + 1].contains("mcp__cormux__get_thread"));
        assert!(!args[at + 1].contains("create_thread"));
        let at = args
            .iter()
            .position(|arg| arg == "--append-system-prompt")
            .unwrap();
        assert!(args[at + 1].contains("create_thread"));
    }

    #[test]
    fn argv_resume_appends_session_id() {
        let args = claude_argv(Some("sess-1"), None, false);
        assert!(args.windows(2).any(|w| w == ["--resume", "sess-1"]));
        assert!(!args.contains(&"--model".into()));
    }

    #[test]
    fn argv_model_appends_flag() {
        let args = claude_argv(None, Some("opus"), false);
        assert!(args.windows(2).any(|w| w == ["--model", "opus"]));
    }
}
