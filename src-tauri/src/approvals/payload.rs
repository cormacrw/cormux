use crate::engines::PermissionRequest;
use crate::model::ApprovalPayload;
use crate::transcript::HostEvent;

/// The `tool` an approval row carries when an agent is asking rather than seeking permission.
pub const QUESTION_TOOL: &str = "question";

/// The card the user answers, and the row's `tool`: the tool the agent asks to use.
pub fn card(request: &PermissionRequest) -> (ApprovalPayload, &str) {
    if !request.questions.is_empty() {
        return (
            ApprovalPayload::asking(request.questions.clone()),
            QUESTION_TOOL,
        );
    }
    (
        ApprovalPayload::to_use(&request.tool, request.detail.as_deref()),
        &request.tool,
    )
}

/// The request as the thread records it, with the card it shows.
pub fn event(request: &PermissionRequest, card: ApprovalPayload, auto_approved: bool) -> HostEvent {
    HostEvent::Permission {
        id: request.id.clone(),
        tool_call_id: request.tool_call_id.clone(),
        card: Box::new(card),
        tool: request.tool.clone(),
        role: request.role.clone(),
        detail: request.detail.clone(),
        auto_approved,
    }
}
