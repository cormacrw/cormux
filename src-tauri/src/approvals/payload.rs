use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::engines::{AgentEvent, ToolKind};

/// The `tool` an approval row carries when an agent is asking rather than seeking permission.
pub const QUESTION_TOOL: &str = "question";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionOption {
    pub label: String,
    #[serde(default)]
    pub description: String,
}

/// One question from an `AskUserQuestion` call, in the shape the tool's input uses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentQuestion {
    pub question: String,
    #[serde(default)]
    pub header: String,
    #[serde(default)]
    pub options: Vec<QuestionOption>,
    #[serde(default)]
    pub multi_select: bool,
}

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
        tool_name,
        kind,
        detail,
        ..
    } = event
    else {
        return (ApprovalPayload::default(), "other".into());
    };

    let what = detail.clone().unwrap_or_else(|| tool_name.clone());
    let lower = tool_name.to_lowercase();
    let tool_key = if lower.contains("bash") || lower.contains("run") {
        ToolKind::Execute.as_str()
    } else {
        kind.as_str()
    };
    let payload = ApprovalPayload {
        title: title.clone(),
        what,
        why: tool_name.clone(),
        ..ApprovalPayload::default()
    };
    (payload, tool_key.into())
}

/// The questions in an `AskUserQuestion` call's input. Empty when it doesn't parse, which
/// leaves the call to the ordinary approval card.
pub fn questions_from_input(input: &Value) -> Vec<AgentQuestion> {
    input
        .get("questions")
        .cloned()
        .and_then(|questions| serde_json::from_value(questions).ok())
        .unwrap_or_default()
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
