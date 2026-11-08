use serde::{Deserialize, Serialize};
use specta::Type;

use crate::model::StopReason;

/// The engine a thread runs on. Claude is the only one; the name is kept on threads and in
/// settings so a second engine has somewhere to go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum EngineKind {
    Claude,
}

/// What a tool call does, as an engine understands it, so nothing past the adapter reads meaning
/// from a tool's name or title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ToolRole {
    Read,
    Search,
    Edit {
        verb: EditVerb,
    },
    /// A shell command; `command` is empty when the engine didn't say which.
    Command {
        command: String,
        description: Option<String>,
    },
    Fetch {
        url: String,
    },
    WebSearch {
        query: String,
    },
    Skill {
        name: String,
    },
    /// A tool from an MCP server, both named as words.
    Mcp {
        server: String,
        action: String,
    },
    /// Starts a subagent, whose own calls name this one as their parent.
    Subagent {
        description: Option<String>,
    },
    /// The engine loading the schemas of tools it hasn't used yet.
    ToolLookup,
    /// Switching the session into plan mode, which only older threads did.
    PlanMode,
    Other {
        name: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum EditVerb {
    Edited,
    Created,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ToolCallStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum MessageRole {
    Agent,
    Thought,
}

/// One internal event every adapter translates into (COR-56).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AgentEvent {
    SessionStarted {
        session_id: String,
    },
    MessageChunk {
        role: MessageRole,
        text: String,
    },
    ToolCall {
        id: String,
        /// The command for a `Command`, otherwise the tool as words.
        title: String,
        role: ToolRole,
        status: ToolCallStatus,
        locations: Vec<String>,
        detail: Option<String>,
        /// Shown as an approval card instead of a step: a question, or a plan asking to start.
        #[serde(default)]
        hidden: bool,
        /// The tool call that spawned this one, when a subagent made it.
        #[serde(default)]
        parent_id: Option<String>,
    },
    Plan {
        entries: Vec<PlanStep>,
    },
    Permission {
        id: String,
        tool_call_id: Option<String>,
        title: String,
        /// The tool as words.
        tool: String,
        role: ToolRole,
        detail: Option<String>,
        auto_approved: bool,
    },
    /// Fields an engine fills in after the first `ToolCall`, such as an edit's path.
    ToolCallUpdate {
        id: String,
        title: Option<String>,
        status: Option<ToolCallStatus>,
        locations: Vec<String>,
        /// The end of what the tool printed, capped, for summing up test and build runs.
        #[serde(default)]
        output: Option<String>,
    },
    /// What the agent is waiting on right now, for the thread's live row.
    CurrentTool {
        id: Option<String>,
        title: String,
    },
    Usage {
        used_tokens: u64,
        context_size: u64,
        cost_usd: Option<f64>,
    },
    TurnEnd {
        #[specta(type = String)]
        stop_reason: StopReason,
        error: Option<String>,
    },
    EngineExited {
        code: Option<i32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanStep {
    pub content: String,
    pub status: String,
}
