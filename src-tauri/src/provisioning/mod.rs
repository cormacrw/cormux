mod runner;

pub use runner::{
    ensure_thread_engine, join_thread_provisioning, retry_provisioning,
    run_workspace_provisioning, skip_provisioning_setup, JoinProvisionJob, LeadProvisionJob,
};

use std::time::Duration;

/// Per-command ceiling for worktree setup (spec gap COR-107).
pub const SETUP_COMMAND_TIMEOUT: Duration = Duration::from_secs(3600);

/// Parses repo **Worktree setup** lines: trim, skip blank and `#` comments.
pub fn parse_setup_commands(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_blank_and_comment_lines() {
        let cmds = parse_setup_commands(
            "pnpm install\n\n# copy env\ncp .env.example .env\n  uv sync  \n",
        );
        assert_eq!(
            cmds,
            vec![
                "pnpm install".to_string(),
                "cp .env.example .env".to_string(),
                "uv sync".to_string(),
            ]
        );
    }
}
