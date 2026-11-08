//! Status vocabularies shared by the store, the workflows and the webview. Each keeps the word
//! it is stored and sent as.

use serde::{Deserialize, Serialize};
use specta::Type;

/// The branch a repo's default is taken to be when git can't tell and none was recorded.
pub const FALLBACK_DEFAULT_BRANCH: &str = "main";

/// A workspace's lifecycle. `ready` is set up with nothing run yet (or no threads); running,
/// paused and idle then follow its threads (see `status_from_threads`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceLifecycle {
    Provisioning,
    Ready,
    Running,
    Paused,
    Idle,
    TearingDown,
    ProvisioningFailed,
}

impl WorkspaceLifecycle {
    /// What the store and the webview call it, the same word serde writes.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Provisioning => "provisioning",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Idle => "idle",
            Self::TearingDown => "tearingDown",
            Self::ProvisioningFailed => "provisioningFailed",
        }
    }

    /// A stored status. Anything unrecognised reads as `Ready`.
    pub fn parse(raw: &str) -> Self {
        match raw {
            "provisioning" => Self::Provisioning,
            "running" => Self::Running,
            "paused" => Self::Paused,
            "idle" => Self::Idle,
            "tearingDown" => Self::TearingDown,
            "provisioningFailed" => Self::ProvisioningFailed,
            _ => Self::Ready,
        }
    }

    /// Past setup and not being torn down: its status follows its threads.
    pub fn is_settled(self) -> bool {
        !matches!(
            self,
            Self::Provisioning | Self::TearingDown | Self::ProvisioningFailed
        )
    }
}

/// What a workspace is for. Stored as `lead` or `review`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum WorkspaceKind {
    /// A Lead works on its own branch.
    #[serde(rename = "lead")]
    Lead,
    /// A Reviewer reads somebody's pull request.
    #[serde(rename = "review")]
    PrReview,
}

impl WorkspaceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lead => "lead",
            Self::PrReview => "review",
        }
    }

    pub fn parse(raw: Option<&str>) -> Self {
        match raw {
            Some("review") => Self::PrReview,
            _ => Self::Lead,
        }
    }

    /// The role of the thread the workspace starts with.
    pub fn own_role(self) -> ThreadRole {
        match self {
            Self::Lead => ThreadRole::Lead,
            Self::PrReview => ThreadRole::Reviewer,
        }
    }
}

/// What a workspace's own thread does there. Threads without one (a scratch's, or helpers from
/// older builds) are stored with no role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ThreadRole {
    Lead,
    Reviewer,
}

impl ThreadRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lead => "lead",
            Self::Reviewer => "reviewer",
        }
    }

    pub fn parse(raw: Option<&str>) -> Option<Self> {
        match raw {
            Some("lead") => Some(Self::Lead),
            Some("reviewer") => Some(Self::Reviewer),
            _ => None,
        }
    }

    /// The thread's title when it's created; only ever shown.
    pub fn title(self) -> &'static str {
        match self {
            Self::Lead => "Lead",
            Self::Reviewer => "Reviewer",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ThreadStatus {
    Provisioning,
    Running,
    Paused,
    Idle,
}

impl ThreadStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Provisioning => "provisioning",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Idle => "idle",
        }
    }

    /// A stored status. Anything unrecognised isn't doing anything, so it reads as `Idle`.
    pub fn parse(raw: &str) -> Self {
        match raw {
            "provisioning" => Self::Provisioning,
            "running" => Self::Running,
            "paused" => Self::Paused,
            _ => Self::Idle,
        }
    }

    /// An agent at work or starting: the workspace reads as running, and its branch stays where
    /// it is. The webview gets the list as `WORKING_THREAD_STATUSES`.
    pub const WORKING: [Self; 2] = [Self::Running, Self::Provisioning];

    pub fn is_working(self) -> bool {
        Self::WORKING.contains(&self)
    }

    /// A turn was started and hasn't ended: it is running, or paused partway, so it can be
    /// paused or resumed. The webview gets the list as `MID_TURN_THREAD_STATUSES`.
    pub const MID_TURN: [Self; 2] = [Self::Running, Self::Paused];

    pub fn is_mid_turn(self) -> bool {
        Self::MID_TURN.contains(&self)
    }

    /// Still has work to finish: working, or paused partway through a turn.
    pub fn is_unfinished(self) -> bool {
        self.is_working() || self.is_mid_turn()
    }

    pub const ALL: [Self; 4] = [Self::Provisioning, Self::Running, Self::Paused, Self::Idle];
}

/// A workspace's status given its threads: the one derivation, for the live record and the
/// stored row alike. Setup and teardown own the workspace until they finish, so their states
/// are left alone. A workspace whose only unfinished threads are paused reads as paused.
pub fn status_from_threads(
    current: WorkspaceLifecycle,
    threads: impl Iterator<Item = ThreadStatus>,
) -> WorkspaceLifecycle {
    if !current.is_settled() {
        return current;
    }
    let threads: Vec<_> = threads.collect();
    if threads.iter().any(|status| status.is_working()) {
        WorkspaceLifecycle::Running
    } else if threads.contains(&ThreadStatus::Paused) {
        WorkspaceLifecycle::Paused
    } else if threads.is_empty() {
        WorkspaceLifecycle::Ready
    } else {
        WorkspaceLifecycle::Idle
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
}

impl ApprovalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Denied => "denied",
        }
    }

    /// A stored status. Anything unrecognised was answered somehow, so it reads as `Denied`.
    pub fn parse(raw: &str) -> Self {
        match raw {
            "pending" => Self::Pending,
            "approved" => Self::Approved,
            _ => Self::Denied,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FindingStatus {
    Open,
    /// Sent to a thread to fix.
    Sent,
    Fixed,
    /// Kept in the list, but not sent or posted.
    Dismissed,
}

impl FindingStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Sent => "sent",
            Self::Fixed => "fixed",
            Self::Dismissed => "dismissed",
        }
    }

    /// A stored status. Anything unrecognised reads as `Open`, so it can still be acted on.
    pub fn parse(raw: &str) -> Self {
        match raw {
            "sent" => Self::Sent,
            "fixed" => Self::Fixed,
            "dismissed" => Self::Dismissed,
            _ => Self::Open,
        }
    }
}

/// What an approval card shows, and how it was answered. Stored as JSON in the approval's row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalPayload {
    pub title: String,
    #[serde(default)]
    pub what: String,
    #[serde(default)]
    pub why: String,
    pub ok_label: String,
    pub no_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub resolved_at_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub deny_reason: Option<String>,
    /// What the agent asked; empty for an ordinary permission request.
    #[serde(default)]
    pub questions: Vec<AgentQuestion>,
    /// The user's answers, keyed by question text.
    #[serde(default)]
    pub answers: std::collections::BTreeMap<String, String>,
}

#[cfg(test)]
impl ApprovalPayload {
    pub fn titled(title: &str) -> Self {
        Self {
            title: title.into(),
            what: String::new(),
            why: String::new(),
            ok_label: "Approve".into(),
            no_label: "Deny".into(),
            resolved_at_ms: None,
            deny_reason: None,
            questions: Vec::new(),
            answers: Default::default(),
        }
    }
}

/// One question an agent asks the user, in the shape Claude's question tool takes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct QuestionOption {
    pub label: String,
    #[serde(default)]
    pub description: String,
}

/// Why a turn ended. Stored and sent as a plain string: `end_turn` when it finished, otherwise
/// the CLI's error subtype.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum StopReason {
    EndTurn,
    Failed(String),
}

impl StopReason {
    const END_TURN: &str = "end_turn";

    /// A failed turn, under the CLI's subtype for it when it gave one.
    pub fn failed(subtype: Option<String>) -> Self {
        Self::Failed(subtype.unwrap_or_else(|| "error".into()))
    }
}

impl From<String> for StopReason {
    fn from(raw: String) -> Self {
        if raw == Self::END_TURN {
            Self::EndTurn
        } else {
            Self::Failed(raw)
        }
    }
}

impl From<StopReason> for String {
    fn from(reason: StopReason) -> Self {
        match reason {
            StopReason::EndTurn => StopReason::END_TURN.into(),
            StopReason::Failed(subtype) => subtype,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIFECYCLES: [WorkspaceLifecycle; 7] = [
        WorkspaceLifecycle::Provisioning,
        WorkspaceLifecycle::Ready,
        WorkspaceLifecycle::Running,
        WorkspaceLifecycle::Paused,
        WorkspaceLifecycle::Idle,
        WorkspaceLifecycle::TearingDown,
        WorkspaceLifecycle::ProvisioningFailed,
    ];

    #[test]
    fn every_lifecycle_survives_the_store_under_its_wire_name() {
        for lifecycle in LIFECYCLES {
            assert_eq!(WorkspaceLifecycle::parse(lifecycle.as_str()), lifecycle);
            assert_eq!(
                serde_json::to_value(lifecycle).unwrap(),
                lifecycle.as_str(),
                "the webview and the store must agree"
            );
        }
        assert_eq!(
            WorkspaceLifecycle::parse("archived"),
            WorkspaceLifecycle::Ready
        );
    }

    #[test]
    fn thread_and_approval_statuses_keep_their_stored_words() {
        for (status, word) in [
            (ThreadStatus::Provisioning, "provisioning"),
            (ThreadStatus::Running, "running"),
            (ThreadStatus::Paused, "paused"),
            (ThreadStatus::Idle, "idle"),
        ] {
            assert_eq!(status.as_str(), word);
            assert_eq!(ThreadStatus::parse(word), status);
            assert_eq!(serde_json::to_value(status).unwrap(), word);
        }
        for status in [
            ApprovalStatus::Pending,
            ApprovalStatus::Approved,
            ApprovalStatus::Denied,
        ] {
            assert_eq!(ApprovalStatus::parse(status.as_str()), status);
            assert_eq!(serde_json::to_value(status).unwrap(), status.as_str());
        }
    }

    #[test]
    fn a_workspace_status_comes_from_its_threads() {
        use ThreadStatus::{Idle, Paused, Provisioning, Running};
        use WorkspaceLifecycle as Lifecycle;
        for (threads, expected) in [
            (vec![], Lifecycle::Ready),
            (vec![Idle], Lifecycle::Idle),
            (vec![Paused], Lifecycle::Paused),
            (vec![Idle, Paused], Lifecycle::Paused),
            (vec![Paused, Running], Lifecycle::Running),
            (vec![Idle, Running], Lifecycle::Running),
            (vec![Idle, Provisioning], Lifecycle::Running),
        ] {
            assert_eq!(
                status_from_threads(Lifecycle::Running, threads.clone().into_iter()),
                expected,
                "{threads:?}"
            );
        }
    }

    #[test]
    fn setup_and_teardown_keep_their_status_whatever_the_threads_do() {
        use WorkspaceLifecycle as Lifecycle;
        for current in [
            Lifecycle::Provisioning,
            Lifecycle::TearingDown,
            Lifecycle::ProvisioningFailed,
        ] {
            assert_eq!(
                status_from_threads(current, [ThreadStatus::Running].into_iter()),
                current
            );
        }
    }

    #[test]
    fn kinds_and_roles_keep_their_stored_words() {
        for kind in [WorkspaceKind::Lead, WorkspaceKind::PrReview] {
            assert_eq!(WorkspaceKind::parse(Some(kind.as_str())), kind);
            assert_eq!(serde_json::to_value(kind).unwrap(), kind.as_str());
        }
        assert_eq!(WorkspaceKind::parse(None), WorkspaceKind::Lead);
        for role in [ThreadRole::Lead, ThreadRole::Reviewer] {
            assert_eq!(ThreadRole::parse(Some(role.as_str())), Some(role));
            assert_eq!(serde_json::to_value(role).unwrap(), role.as_str());
        }
        assert_eq!(ThreadRole::parse(Some("Lead")), None);
    }

    #[test]
    fn stop_reasons_keep_their_stored_words() {
        for (reason, word) in [
            (StopReason::EndTurn, "end_turn"),
            (StopReason::failed(None), "error"),
            (
                StopReason::failed(Some("error_max_turns".into())),
                "error_max_turns",
            ),
        ] {
            assert_eq!(serde_json::to_value(&reason).unwrap(), word);
            assert_eq!(
                serde_json::from_value::<StopReason>(word.into()).unwrap(),
                reason
            );
        }
    }
}
