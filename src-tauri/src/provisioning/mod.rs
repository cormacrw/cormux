//! Bringing a workspace into being: its records, then in the background its worktree, setup
//! commands and first turn, as the caller's `ProvisionPlan` says.

mod runner;

pub use runner::{ProvisionPlan, WorkspaceSpec, create, fail, retry, run, skip_setup, start};

use std::time::Duration;

pub const SETUP_COMMAND_TIMEOUT: Duration = Duration::from_secs(3600);

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
        let cmds =
            parse_setup_commands("pnpm install\n\n# copy env\ncp .env.example .env\n  uv sync  \n");
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
