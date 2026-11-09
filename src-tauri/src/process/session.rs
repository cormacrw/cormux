use std::fmt;
use std::path::PathBuf;

/// A PTY session: the workspace whose teardown stops it, if any, and a name unique within it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId {
    owner: Option<String>,
    name: String,
}

impl SessionId {
    /// A session that goes with `workspace`.
    pub fn of_workspace(workspace: &str, name: impl Into<String>) -> Self {
        Self {
            owner: Some(workspace.to_string()),
            name: name.into(),
        }
    }

    /// A session no workspace owns.
    pub fn unowned(name: impl Into<String>) -> Self {
        Self {
            owner: None,
            name: name.into(),
        }
    }

    /// The workspace whose teardown stops this session, if any.
    pub fn owner_workspace(&self) -> Option<&str> {
        self.owner.as_deref()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.owner {
            Some(owner) => write!(f, "{owner}-{}", self.name),
            None => f.write_str(&self.name),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sessions_know_their_workspace_without_parsing_names() {
        // A workspace whose id ends like another's session name still owns only its own.
        let setup = SessionId::of_workspace("ws-app", "setup-0");
        assert_eq!(setup.owner_workspace(), Some("ws-app"));
        let run = SessionId::of_workspace("ws", "app");
        assert_eq!(run.to_string(), "ws-app");
        assert_eq!(run.owner_workspace(), Some("ws"));
        let test = SessionId::unowned("test-setup-ws-1");
        assert_eq!(test.owner_workspace(), None);
    }
}
