use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub use crate::engines::AgentQuestion;
use crate::engines::{AgentEvent, ToolRole};

/// The `tool` an approval row carries when an agent is asking rather than seeking permission.
pub const QUESTION_TOOL: &str = "question";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalPayload {
    pub title: String,
    pub what: String,
    pub why: String,
    pub ok_label: String,
    pub no_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny_reason: Option<String>,
    /// What the agent asked; empty for an ordinary permission request.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub questions: Vec<AgentQuestion>,
    /// The user's answers, keyed by question text.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub answers: BTreeMap<String, String>,
}

impl Default for ApprovalPayload {
    fn default() -> Self {
        Self {
            title: "Needs approval".into(),
            what: String::new(),
            why: String::new(),
            ok_label: "Approve".into(),
            no_label: "Deny".into(),
            resolved_at_ms: None,
            deny_reason: None,
            questions: Vec::new(),
            answers: BTreeMap::new(),
        }
    }
}

pub fn payload_from_permission(event: &AgentEvent) -> (ApprovalPayload, String) {
    let AgentEvent::Permission {
        title,
        tool,
        role,
        detail,
        ..
    } = event
    else {
        return (ApprovalPayload::default(), "other".into());
    };
    let payload = ApprovalPayload {
        title: title.clone(),
        what: detail.clone().unwrap_or_else(|| tool.clone()),
        why: tool.clone(),
        ..ApprovalPayload::default()
    };
    (payload, tool_key(role).into())
}

/// The approval row's `tool`: the kind of thing it would do.
fn tool_key(role: &ToolRole) -> &'static str {
    match role {
        ToolRole::Read => "read",
        ToolRole::Search | ToolRole::WebSearch { .. } => "search",
        ToolRole::Edit { .. } => "edit",
        ToolRole::Command { .. } => "execute",
        ToolRole::Fetch { .. } => "fetch",
        _ => "other",
    }
}

pub fn payload_with_questions(
    payload: ApprovalPayload,
    questions: Vec<AgentQuestion>,
) -> ApprovalPayload {
    ApprovalPayload {
        title: "Question".into(),
        what: questions
            .first()
            .map(|question| question.question.clone())
            .unwrap_or_default(),
        why: String::new(),
        ok_label: "Send".into(),
        no_label: "Skip".into(),
        questions,
        ..payload
    }
}
