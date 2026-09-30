use serde::{Deserialize, Serialize};

use crate::engines::{AgentEvent, ToolKind};

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
    if lower.contains("plan") || title.to_lowercase().contains("plan") {
        return (
            "execute_plan".into(),
            LabelSet {
                title: "Execute plan".into(),
                what: detail.to_string(),
                why: "Nothing is edited until you approve.".into(),
                ok: "Approve plan".into(),
                no: "Revise".into(),
                effect: Some("execute_plan".into()),
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

pub fn payload_with_option_labels(
    mut payload: ApprovalPayload,
    ok_label: Option<String>,
    no_label: Option<String>,
) -> ApprovalPayload {
    if let Some(ok) = ok_label.filter(|value| !value.is_empty()) {
        payload.ok_label = ok;
    }
    if let Some(no) = no_label.filter(|value| !value.is_empty()) {
        payload.no_label = no;
    }
    payload
}
