mod acp;
mod claude;
mod command;
mod detect;
mod events;
mod manager;
mod models;

pub use detect::EngineStatus;
#[allow(unused_imports)]
pub use events::{AgentEvent, EngineKind, MessageRole, PlanStep, ToolCallStatus, ToolKind};
#[allow(unused_imports)]
pub use manager::{EngineRegistry, SpawnSpec, default_acp_argv};
pub use models::ThreadModels;

#[allow(unused_imports)]
pub use claude::{ClaudeSession, Event, PermissionDecision, SpawnOptions, claude_argv};
