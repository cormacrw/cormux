use serde::{Deserialize, Serialize};
use specta::Type;

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

/// What an engine said, as every adapter translates it; see `EngineOutput` for the rest of what
/// it reports.
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanStep {
    pub content: String,
    pub status: PlanStepStatus,
}

/// Where a plan step stands. Rows saved before this was typed hold `in_progress`; a word this
/// build doesn't know reads as not started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum PlanStepStatus {
    #[serde(alias = "in_progress")]
    InProgress,
    Completed,
    #[serde(other)]
    Pending,
}
