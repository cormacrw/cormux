mod acp;
mod claude;
mod command;
mod detect;
mod events;
mod manager;
mod models;

pub use detect::EngineStatus;
pub use events::{AgentEvent, EngineKind, MessageRole, ToolCallStatus, ToolKind};
pub use manager::{EngineRegistry, SpawnSpec};
pub use models::ThreadModels;

pub use claude::claude_argv;
