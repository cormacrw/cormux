use serde::Serialize;
use specta::Type;

pub type Result<T> = std::result::Result<T, Error>;

/// Shared error type for the Rust core. Serialised as a tagged union so the
/// generated TypeScript bindings stay in lockstep with Rust.
#[derive(Debug, thiserror::Error, Serialize, Type)]
#[serde(tag = "kind", content = "message")]
pub enum Error {
    #[error("io: {0}")]
    Io(String),

    #[error("shell environment: {0}")]
    ShellEnv(String),

    #[error("git: {0}")]
    Git(String),

    #[error("git conflict during {operation}")]
    GitConflict {
        operation: crate::model::GitConflictOperation,
        paths: Vec<String>,
    },

    #[error("workspace: {0}")]
    Workspace(String),

    #[error("engine: {0}")]
    Engine(String),

    #[error("approval: {0}")]
    Approval(String),

    #[error("mcp: {0}")]
    Mcp(String),

    #[error("process: {0}")]
    Process(String),

    #[error("github: {0}")]
    Github(String),

    #[error("clickup: {0}")]
    Clickup(String),

    #[error("llm: {0}")]
    Llm(String),

    #[error("store: {0}")]
    Store(String),

    #[error("metrics: {0}")]
    Metrics(String),
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

impl Error {
    pub fn unknown_workspace(workspace_id: &str) -> Self {
        Self::Workspace(format!("unknown workspace {workspace_id}"))
    }

    pub fn unknown_repo(repo_id: &str) -> Self {
        Self::Workspace(format!("unknown repo {repo_id}"))
    }
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        Self::Store(error.to_string())
    }
}
