//! Agent engines behind one interface: launch one with a [`LaunchConfig`], send it
//! [`EngineCommand`]s, and read back engine-neutral [`EngineOutput`]. What an engine's own
//! vocabulary means (its tool names, how it names MCP tools, its CLI flags) stays in its adapter.

mod claude;
mod command;
mod detect;
mod events;
mod models;
pub mod prompt;

use std::collections::BTreeMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

pub use claude::ClaudeEngines;
pub use command::EngineCommand;
pub use detect::{EngineStatus, detect_engines};
pub use events::{AgentEvent, AgentQuestion, MessageRole, ToolRole};
pub use models::{ModelOption, ThreadModels, static_efforts, static_models};

use crate::error::Result;

/// How to start a session.
#[derive(Debug, Clone, Default)]
pub struct LaunchConfig {
    /// The thread the session serves, for its logs.
    pub thread_id: String,
    pub cwd: PathBuf,
    pub model: Option<String>,
    pub effort: Option<String>,
    /// The session to carry on, if any.
    pub resume: Option<String>,
    /// Told to the agent once, at the start of the session.
    pub instructions: String,
    pub host_tools: Option<HostToolsConfig>,
}

/// Tools the host serves itself, over the session's own connection.
#[derive(Debug, Clone, Default)]
pub struct HostToolsConfig {
    pub server_name: String,
    /// Tools that run without asking the user.
    pub allowed: Vec<String>,
    /// Tools the session must not offer.
    pub disallowed: Vec<String>,
    /// What the agent is told about the tools.
    pub instructions: String,
}

/// A running engine: where its commands go, and its process group so quitting can kill it.
pub struct EngineHandle {
    pub commands: mpsc::UnboundedSender<EngineCommand>,
    pub pgid: Option<u32>,
}

/// What an engine reports, in the order it happened.
#[derive(Debug)]
pub enum EngineOutput {
    Event(AgentEvent),
    /// The tokens a turn left in context, and what that turn cost on its own.
    Usage {
        used_tokens: u64,
        context_size: u64,
        turn_cost_usd: Option<f64>,
    },
    /// The engine waits for the answer.
    Permission(PermissionRequest),
    /// A message for one of the host's tool servers, waiting for its reply.
    HostTool(HostToolCall),
    /// The engine process is gone.
    Exited,
}

/// An agent asking to use a tool, or asking the user something.
#[derive(Debug)]
pub struct PermissionRequest {
    pub id: String,
    pub tool_call_id: Option<String>,
    /// The tool as words.
    pub tool: String,
    pub role: ToolRole,
    pub detail: Option<String>,
    /// What the agent is asking; empty for a request to use a tool.
    pub questions: Vec<AgentQuestion>,
    /// Dropped unanswered once the turn is over, when the engine has stopped waiting.
    pub respond: oneshot::Sender<PermissionAnswer>,
}

impl PermissionRequest {
    pub fn event(&self, auto_approved: bool) -> AgentEvent {
        AgentEvent::Permission {
            id: self.id.clone(),
            tool_call_id: self.tool_call_id.clone(),
            title: format!("Use {}", self.tool),
            tool: self.tool.clone(),
            role: self.role.clone(),
            detail: self.detail.clone(),
            auto_approved,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionAnswer {
    Allow,
    Deny {
        message: String,
    },
    /// The user's answers, keyed by question text.
    Answer {
        answers: BTreeMap<String, String>,
    },
}

#[derive(Debug)]
pub struct HostToolCall {
    pub server: String,
    /// One JSON-RPC message.
    pub message: Value,
    pub respond: oneshot::Sender<Value>,
}

pub type Launching<'a> = Pin<Box<dyn Future<Output = Result<EngineHandle>> + Send + 'a>>;

/// Starts engines. The app launches Claude; tests launch fakes.
pub trait Engines: Send + Sync {
    /// Starts a session that reports through `output`. The caller is already reading it: a
    /// session can need its host's tools answered before it's up.
    fn launch(
        &self,
        config: LaunchConfig,
        output: mpsc::UnboundedSender<EngineOutput>,
    ) -> Launching<'_>;
}
