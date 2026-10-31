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
    pub effect: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny_reason: Option<String>,
    /// The markdown plan an `ExitPlanMode` call hands over; the Plan tab shows it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
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
            effect: None,
            resolved_at_ms: None,
            deny_reason: None,
            plan: None,
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
    let (tool_key, labels) = harness_tool_meta(tool_name, *kind, title, &what);
    let mut payload = ApprovalPayload {
        title: labels.title,
        what: labels.what,
        why: labels.why,
        ok_label: labels.ok,
        no_label: labels.no,
        effect: labels.effect,
        ..ApprovalPayload::default()
    };
    if payload.what.is_empty() {
        payload.what = what;
    }
    (payload, tool_key)
}

struct LabelSet {
    title: String,
    what: String,
    why: String,
    ok: String,
    no: String,
    effect: Option<String>,
}

fn harness_tool_meta(
    tool_name: &str,
    kind: ToolKind,
    title: &str,
    detail: &str,
) -> (String, LabelSet) {
    let lower = tool_name.to_lowercase();
    if lower.contains("delete") || kind == ToolKind::Delete {
        return (
            "delete_file".into(),
            LabelSet {
                title: "Delete file".into(),
                what: detail.to_string(),
                why: "Agent requested a delete".into(),
                ok: "Approve delete".into(),
                no: "Keep file".into(),
                effect: Some("delete_file".into()),
            },
        );
    }
    if lower.contains("migration") || lower.contains("migrate") {
        return (
            "migration".into(),
            LabelSet {
                title: "Run database migration".into(),
                what: detail.to_string(),
                why: "Schema change requested".into(),
                ok: "Run migration".into(),
                no: "Skip".into(),
                effect: Some("migration".into()),
            },
        );
    }
    if lower.contains("suggestion") || lower.contains("review") {
        return (
            "send_suggestions".into(),
            LabelSet {
                title: "Apply review suggestions".into(),
                what: detail.to_string(),
                why: title.to_string(),
                ok: "Send to Lead".into(),
                no: "Dismiss".into(),
                effect: Some("send_to_lead".into()),
            },
        );
    }
    if kind == ToolKind::Execute || lower.contains("bash") || lower.contains("run") {
        return (
            "execute".into(),
            LabelSet {
                title: title.to_string(),
                what: detail.to_string(),
                why: tool_name.to_string(),
                ok: "Approve".into(),
                no: "Deny".into(),
                effect: None,
            },
        );
    }
    (
        kind.as_str().into(),
        LabelSet {
            title: title.to_string(),
            what: detail.to_string(),
            why: tool_name.to_string(),
            ok: "Approve".into(),
            no: "Deny".into(),
            effect: None,
        },
    )
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
