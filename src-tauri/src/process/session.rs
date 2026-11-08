use std::fmt;
use std::path::PathBuf;

/// A PTY session, named for what it runs and whom it belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SessionId {
    /// A workspace's `index`th worktree setup command.
    Setup { workspace: String, index: usize },
    /// A workspace's app (its run command).
    Run { workspace: String },
    /// A repo's `index`th setup command, tried in its own checkout from Settings.
    RepoTest { repo: String, index: usize },
}

impl SessionId {
    /// The workspace whose teardown stops this session, if any.
    pub fn owner_workspace(&self) -> Option<&str> {
        match self {
            Self::Setup { workspace, .. } | Self::Run { workspace } => Some(workspace),
            Self::RepoTest { .. } => None,
        }
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Setup { workspace, index } => write!(f, "{workspace}-setup-{index}"),
            Self::Run { workspace } => write!(f, "{workspace}-app"),
            Self::RepoTest { repo, index } => write!(f, "test-setup-{repo}-{index}"),
        }
    }
}

/// A command line run by the user's login shell (`$SHELL -l -c cmd`).
#[derive(Debug, Clone)]
pub struct ShellCommand {
    pub cmd: String,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
}

/// What a setup or run command is told about the checkout it runs in.
#[derive(Debug, Clone)]
pub struct HarnessEnv {
    pub repo_path: String,
    pub worktree_path: String,
    pub workspace: String,
    pub branch: String,
    pub base: String,
}

impl HarnessEnv {
    pub fn vars(&self) -> Vec<(String, String)> {
        vec![
            ("HARNESS_REPO_PATH".into(), self.repo_path.clone()),
            ("HARNESS_WORKTREE_PATH".into(), self.worktree_path.clone()),
            ("HARNESS_WORKSPACE".into(), self.workspace.clone()),
            ("HARNESS_BRANCH".into(), self.branch.clone()),
            ("HARNESS_BASE".into(), self.base.clone()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sessions_know_their_workspace_without_parsing_names() {
        // A workspace whose id ends like another's session name still owns only its own.
        let setup = SessionId::Setup {
            workspace: "ws-app".into(),
            index: 0,
        };
        assert_eq!(setup.owner_workspace(), Some("ws-app"));
        let run = SessionId::Run {
            workspace: "ws".into(),
        };
        assert_eq!(run.to_string(), "ws-app");
        assert_eq!(run.owner_workspace(), Some("ws"));
        let test = SessionId::RepoTest {
            repo: "ws".into(),
            index: 1,
        };
        assert_eq!(test.owner_workspace(), None);
    }
}
