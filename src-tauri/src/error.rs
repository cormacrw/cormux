use serde::Serialize;

pub type Result<T> = std::result::Result<T, Error>;

/// Shared error type for the Rust core. Command handlers map this to a string
/// for the webview; domain modules add variants as they grow past stubs.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("shell environment: {0}")]
    ShellEnv(String),

    #[error("git: {0}")]
    Git(String),

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

    #[error("llm: {0}")]
    Llm(String),

    #[error("store: {0}")]
    Store(String),

    #[error("metrics: {0}")]
    Metrics(String),

    #[error("{0} is not implemented yet")]
    NotImplemented(&'static str),
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
