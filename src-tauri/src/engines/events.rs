use serde::{Deserialize, Serialize};
use specta::Type;

/// Engines Harness can spawn. Launch is Claude and Cursor; Codex and Gemini share ACP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum EngineKind {
    Claude,
    Cursor,
    Codex,
    Gemini,
}

impl EngineKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Cursor => "cursor",
            Self::Codex => "codex",
            Self::Gemini => "gemini",
        }
    }

    /// Candidate binaries on PATH, first match wins.
    pub fn binaries(self) -> &'static [&'static str] {
        match self {
            Self::Claude => &["claude"],
            Self::Cursor => &["agent"],
            Self::Codex => &["codex-acp", "codex"],
            Self::Gemini => &["gemini"],
        }
    }

    pub fn all() -> &'static [EngineKind] {
        &[Self::Claude, Self::Cursor, Self::Codex, Self::Gemini]
    }
}

/// ACP `ToolKind`, owned so adapters and IPC do not depend on the protocol crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ToolKind {
    Read,
    Edit,
    Delete,
    Move,
    Search,
    Execute,
    Think,
    Fetch,
    Other,
}

impl ToolKind {
    pub fn is_readonly(self) -> bool {
        matches!(self, Self::Read | Self::Search | Self::Think)
    }

    pub fn from_claude_tool(name: &str) -> Self {
        match name {
            "Read" | "LS" | "NotebookRead" => Self::Read,
            "Glob" | "Grep" | "WebSearch" => Self::Search,
            "Edit" | "Write" | "NotebookEdit" => Self::Edit,
            "Bash" | "BashOutput" => Self::Execute,
            "WebFetch" => Self::Fetch,
            _ => Self::Other,
        }
    }
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
    User,
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
        title: String,
        name: Option<String>,
        kind: ToolKind,
        status: ToolCallStatus,
        locations: Vec<String>,
        detail: Option<String>,
    },
    Plan {
        entries: Vec<PlanStep>,
    },
    Permission {
        id: String,
        tool_call_id: Option<String>,
        title: String,
        tool_name: String,
        kind: ToolKind,
        detail: Option<String>,
        auto_approved: bool,
    },
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
        stop_reason: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_tool_names_map_to_acp_kinds() {
        assert_eq!(ToolKind::from_claude_tool("Read"), ToolKind::Read);
        assert_eq!(ToolKind::from_claude_tool("Grep"), ToolKind::Search);
        assert_eq!(ToolKind::from_claude_tool("Bash"), ToolKind::Execute);
        assert_eq!(ToolKind::from_claude_tool("Write"), ToolKind::Edit);
        assert!(ToolKind::Read.is_readonly());
        assert!(!ToolKind::Execute.is_readonly());
    }
}
