use crate::engines::{PermissionRequest, ToolRole};
use crate::model::ApprovalPayload;
use crate::transcript::HostEvent;

/// The `tool` an approval row carries when an agent is asking rather than seeking permission.
pub const QUESTION_TOOL: &str = "question";

/// The card the user answers, and the row's `tool`: the kind of thing it would do.
pub fn card(request: &PermissionRequest) -> (ApprovalPayload, &'static str) {
    if let Some(first) = request.questions.first() {
        let payload = ApprovalPayload {
            title: "Question".into(),
            what: first.question.clone(),
            why: String::new(),
            ok_label: "Send".into(),
            no_label: "Skip".into(),
            resolved_at_ms: None,
            deny_reason: None,
            questions: request.questions.clone(),
            answers: Default::default(),
        };
        return (payload, QUESTION_TOOL);
    }
    let payload = ApprovalPayload {
        title: format!("Use {}", request.tool),
        what: request
            .detail
            .clone()
            .unwrap_or_else(|| request.tool.clone()),
        why: request.tool.clone(),
        ok_label: "Approve".into(),
        no_label: "Deny".into(),
        resolved_at_ms: None,
        deny_reason: None,
        questions: Vec::new(),
        answers: Default::default(),
    };
    (payload, tool_key(&request.role))
}

/// The request as the thread records it.
pub fn event(request: &PermissionRequest, auto_approved: bool) -> HostEvent {
    HostEvent::Permission {
        id: request.id.clone(),
        tool_call_id: request.tool_call_id.clone(),
        title: format!("Use {}", request.tool),
        tool: request.tool.clone(),
        role: request.role.clone(),
        detail: request.detail.clone(),
        auto_approved,
    }
}

fn tool_key(role: &ToolRole) -> &'static str {
    match role {
        ToolRole::Read => "read",
        ToolRole::Search | ToolRole::WebSearch { .. } => "search",
        ToolRole::Edit { .. } => "edit",
        ToolRole::Command { .. } => "execute",
        ToolRole::Fetch { .. } => "fetch",
        ToolRole::Skill { .. }
        | ToolRole::Mcp { .. }
        | ToolRole::Subagent { .. }
        | ToolRole::ToolLookup
        | ToolRole::PlanMode
        | ToolRole::Other { .. } => "other",
    }
}
