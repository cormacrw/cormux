//! Status vocabularies shared by the store, the workflows and the webview. Each keeps the word
//! it is stored and sent as, so typed code reads and writes the same rows as before.

use std::fmt;

use serde::{Deserialize, Serialize};
use specta::Type;

/// A workspace's lifecycle. `ready` is idle with no threads; running/idle describe activity
/// after provisioning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceLifecycle {
    Provisioning,
    Ready,
    Running,
    Idle,
    TearingDown,
    ProvisioningFailed,
}

impl WorkspaceLifecycle {
    /// What the store and the webview call it, the same word serde writes.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Provisioning => "provisioning",
            Self::Ready => "ready",
            Self::Running => "running",
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
            "idle" => Self::Idle,
            "tearingDown" => Self::TearingDown,
            "provisioningFailed" => Self::ProvisioningFailed,
            _ => Self::Ready,
        }
    }
}

/// What a thread is doing. `ThreadRow::status` stays a string, since that's what the webview
/// is sent; these are the only values Rust writes to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    /// A stored status. Anything else, such as the `waiting` older builds could leave behind,
    /// isn't doing anything, so it reads as `Idle`.
    pub fn parse(raw: &str) -> Self {
        match raw {
            "provisioning" => Self::Provisioning,
            "running" => Self::Running,
            "paused" => Self::Paused,
            _ => Self::Idle,
        }
    }
}

impl From<ThreadStatus> for String {
    fn from(status: ThreadStatus) -> Self {
        status.as_str().to_string()
    }
}

/// A workspace's status given its threads: the one derivation, for the live record and the
/// stored row alike. Setup and teardown own the workspace until they finish, so their states
/// are left alone.
pub fn status_from_threads(
    current: WorkspaceLifecycle,
    threads: impl Iterator<Item = ThreadStatus>,
) -> WorkspaceLifecycle {
    if matches!(
        current,
        WorkspaceLifecycle::Provisioning
            | WorkspaceLifecycle::TearingDown
            | WorkspaceLifecycle::ProvisioningFailed
    ) {
        return current;
    }
    let threads: Vec<_> = threads.collect();
    if threads
        .iter()
        .any(|activity| matches!(activity, ThreadStatus::Running | ThreadStatus::Provisioning))
    {
        WorkspaceLifecycle::Running
    } else if threads.is_empty() {
        WorkspaceLifecycle::Ready
    } else {
        WorkspaceLifecycle::Idle
    }
}

/// Where an approval stands. `ApprovalRow::status` stays a string for the webview.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

impl From<ApprovalStatus> for String {
    fn from(status: ApprovalStatus) -> Self {
        status.as_str().to_string()
    }
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

/// The git operation a conflict stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum GitConflictOperation {
    Merge,
    Rebase,
}

impl fmt::Display for GitConflictOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Merge => "merge",
            Self::Rebase => "rebase",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIFECYCLES: [WorkspaceLifecycle; 6] = [
        WorkspaceLifecycle::Provisioning,
        WorkspaceLifecycle::Ready,
        WorkspaceLifecycle::Running,
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
    fn thread_statuses_keep_their_stored_words() {
        for (status, word) in [
            (ThreadStatus::Provisioning, "provisioning"),
            (ThreadStatus::Running, "running"),
            (ThreadStatus::Paused, "paused"),
            (ThreadStatus::Idle, "idle"),
        ] {
            assert_eq!(status.as_str(), word);
            assert_eq!(ThreadStatus::parse(word), status);
        }
        assert_eq!(ThreadStatus::parse("waiting"), ThreadStatus::Idle);
    }

    #[test]
    fn a_workspace_status_comes_from_its_threads() {
        use ThreadStatus::{Idle, Paused, Provisioning, Running};
        use WorkspaceLifecycle as Lifecycle;
        for (threads, expected) in [
            (vec![], Lifecycle::Ready),
            (vec![Idle], Lifecycle::Idle),
            (vec![Paused], Lifecycle::Idle),
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
