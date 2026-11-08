//! What the core itself did in a thread, as the Timeline shows it. Each step is typed by what
//! happened, so the UI picks a step's row and icon from its kind and rewording a title here
//! can't change either. `transcript` stores the title and detail alongside as display text.

use serde::{Deserialize, Serialize};
use specta::Type;
use uuid::Uuid;

/// How a workspace came to be on another branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BranchChange {
    CheckedOut {
        previous: String,
    },
    Created {
        previous: String,
    },
    /// Noticed on disk: something other than Cormux moved the worktree.
    CheckedOutOutside {
        previous: String,
    },
    AddedToStack {
        on_top_of: String,
    },
    StackPushed,
    StackSynced,
}

impl BranchChange {
    fn detail(&self, branch: &str) -> String {
        match self {
            Self::CheckedOut { previous } => {
                format!("Checked out {branch} in this worktree, previously {previous}")
            }
            Self::Created { previous } => {
                format!("Created and checked out {branch}, previously {previous}")
            }
            Self::CheckedOutOutside { previous } => {
                format!("Checked out {branch} outside Cormux, previously {previous}")
            }
            Self::AddedToStack { on_top_of } => {
                format!("Added {branch} to the stack on top of {on_top_of}")
            }
            Self::StackPushed => "Pushed the stack".into(),
            Self::StackSynced => "Synced the stack".into(),
        }
    }
}

/// Something the core did in a thread. A new kind fails `pnpm check` until the UI's `STEP_ROW`
/// (`src/lib/thread/timeline/fold.ts`) and `rows/step-icons.ts` give it a row.
/// Fields stay snake_case, like `AgentEvent`'s: specta exports them as written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TimelineStep {
    AgentStopped,
    AgentPaused,
    AgentResumed,
    /// `ended_session` is what the conversation before it can be resumed from.
    SessionStarted {
        ended_session: Option<String>,
    },
    SessionResumed {
        conversation: String,
        ended_session: Option<String>,
    },
    StartedByThread {
        workspace: String,
    },
    MessageFromThread {
        workspace: String,
    },
    ReportFromThread {
        title: String,
    },
    // The next four are no longer recorded. Approving a tool whose name held "delete",
    // "migration", "review" or "suggestion" used to add them, so older threads still show them.
    FileDeleted {
        path: String,
    },
    MigrationApplied,
    SuggestionsForwarded,
    SuggestionsSent {
        approval_id: String,
        what: String,
    },
    BranchSwitched {
        branch: String,
        change: BranchChange,
    },
    CommitsPulled {
        count: u32,
        base: String,
        branch: String,
    },
    BranchRebased {
        branch: String,
        base: String,
        replayed_onto: u32,
    },
    WorktreeCreated {
        branch: String,
        base: String,
    },
    WorktreeReady {
        repo: String,
        setup_commands: u32,
    },
    PrOpened {
        number: i64,
        branch: String,
        base: String,
    },
    PrCheckedOut {
        number: i64,
        head: String,
        base: String,
        files_changed: i64,
    },
    ReviewSubmitted {
        number: i64,
    },
    ReviewFindingsUnreadable {
        reason: String,
    },
    /// The review this thread ran found something; its card opens the Findings panel.
    ReviewFindingsReady {
        count: u32,
    },
    FindingsReceived {
        count: u32,
        titles: Vec<String>,
    },
    FindingsFixed {
        count: u32,
        files: Vec<String>,
    },
    /// A step an older build saved with only its English title, which drew it the way `icon`
    /// and `run` say: between turns, or among the agent's steps.
    Legacy {
        title: String,
        icon: StepIcon,
        run: bool,
    },
}

/// The icons older builds drew their untyped steps with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum StepIcon {
    Pause,
    Play,
    Stop,
    Session,
    Download,
    Branch,
    Pr,
    File,
    Trash,
    Database,
    Check,
    List,
    Tool,
}

fn plural(count: u32) -> &'static str {
    if count == 1 { "" } else { "s" }
}

impl TimelineStep {
    /// Display text only: nothing may read meaning from it.
    pub fn title(&self) -> String {
        match self {
            Self::AgentStopped => "You stopped the agent".into(),
            Self::AgentPaused => "You paused the agent".into(),
            Self::AgentResumed => "You resumed the agent".into(),
            Self::SessionStarted { .. } => "Started a new session".into(),
            Self::SessionResumed { .. } => "Resumed an earlier conversation".into(),
            Self::StartedByThread { workspace } => format!("Started by {workspace}"),
            Self::MessageFromThread { workspace } => format!("Message from {workspace}"),
            Self::ReportFromThread { title } => format!("Report from {title}"),
            Self::FileDeleted { path } => format!("Deleted {path}"),
            Self::MigrationApplied => "Applied migration".into(),
            Self::SuggestionsForwarded => "Sent suggestions to Lead".into(),
            Self::SuggestionsSent { .. } => "Suggestions sent".into(),
            Self::BranchSwitched { branch, .. } => format!("Switched to `{branch}`"),
            Self::CommitsPulled { count, base, .. } => {
                format!("Pulled {count} commit{} from {base}", plural(*count))
            }
            Self::BranchRebased { branch, base, .. } => format!("Rebased {branch} onto {base}"),
            Self::WorktreeCreated { .. } => "Created worktree".into(),
            Self::WorktreeReady { setup_commands, .. } if *setup_commands > 0 => {
                "Ran worktree setup".into()
            }
            Self::WorktreeReady { .. } => "Worktree ready".into(),
            Self::PrOpened { number, .. } => format!("Opened PR #{number}"),
            Self::PrCheckedOut { number, .. } => format!("Checked out #{number}"),
            Self::ReviewSubmitted { number } => format!("Submitted review on #{number}"),
            Self::ReviewFindingsUnreadable { .. } => "Couldn’t read the review findings".into(),
            Self::ReviewFindingsReady { count } => {
                format!("Review found {count} finding{}", plural(*count))
            }
            Self::FindingsReceived { count, .. } => {
                format!("Received {count} findings from the review")
            }
            Self::FindingsFixed { count, .. } => format!("Fixed {count} findings"),
            Self::Legacy { title, .. } => title.clone(),
        }
    }

    pub fn detail(&self) -> Option<String> {
        match self {
            Self::AgentStopped
            | Self::AgentPaused
            | Self::AgentResumed
            | Self::SessionStarted { .. }
            | Self::SessionResumed { .. }
            | Self::StartedByThread { .. }
            | Self::MessageFromThread { .. }
            | Self::ReportFromThread { .. }
            | Self::FileDeleted { .. }
            | Self::MigrationApplied
            | Self::SuggestionsForwarded
            | Self::ReviewFindingsReady { .. }
            | Self::Legacy { .. } => None,
            Self::SuggestionsSent { what, .. } => Some(what.clone()),
            Self::BranchSwitched { branch, change } => Some(change.detail(branch)),
            Self::CommitsPulled { branch, .. } => {
                Some(format!("Merged into {branch} with no conflicts"))
            }
            Self::BranchRebased { replayed_onto, .. } => Some(format!(
                "Replayed local commits on top of {replayed_onto} new commit{} with no conflicts",
                plural(*replayed_onto)
            )),
            Self::WorktreeCreated { branch, base } => Some(format!("{branch} ← {base}")),
            Self::WorktreeReady {
                repo,
                setup_commands,
            } if *setup_commands > 0 => Some(format!(
                "{setup_commands} commands from {repo} settings, see the Output tab"
            )),
            Self::WorktreeReady { repo, .. } => {
                Some(format!("No setup commands configured for {repo}"))
            }
            Self::PrOpened { branch, base, .. } => Some(format!("{branch} → {base}")),
            Self::PrCheckedOut {
                head,
                base,
                files_changed,
                ..
            } => Some(format!("{head} → {base} · {files_changed} files changed")),
            Self::ReviewSubmitted { .. } => Some("Posted to GitHub".into()),
            Self::ReviewFindingsUnreadable { reason } => Some(reason.clone()),
            Self::FindingsReceived { titles, .. } => Some(titles.join(" · ")),
            Self::FindingsFixed { files, .. } => Some(files.join(", ")),
        }
    }

    /// Steps that can be written twice share an id, and the timeline keeps the first.
    pub fn id(&self) -> String {
        match self {
            Self::PrOpened { number, .. } => format!("pr-open-{number}"),
            Self::SuggestionsSent { approval_id, .. } => format!("review-{approval_id}"),
            _ => format!("step-{}", Uuid::new_v4()),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn session_steps_keep_what_resuming_needs() {
        let step = TimelineStep::SessionResumed {
            conversation: "after-4".into(),
            ended_session: Some("sess-1".into()),
        };
        assert_eq!(
            serde_json::to_value(&step).unwrap(),
            json!({ "kind": "sessionResumed", "conversation": "after-4", "ended_session": "sess-1" })
        );
        assert_eq!(step.detail(), None);
    }

    #[test]
    fn only_steps_written_twice_share_an_id() {
        assert_ne!(
            TimelineStep::AgentStopped.id(),
            TimelineStep::AgentStopped.id()
        );
        let sent = || TimelineStep::SuggestionsSent {
            approval_id: "a1".into(),
            what: "src/a.rs".into(),
        };
        assert_eq!(sent().id(), "review-a1");
        assert_eq!(sent().id(), sent().id());
    }

    #[test]
    fn counts_read_in_the_singular_and_the_plural() {
        let pulled = |count| TimelineStep::CommitsPulled {
            count,
            base: "main".into(),
            branch: "b".into(),
        };
        assert_eq!(pulled(1).title(), "Pulled 1 commit from main");
        assert_eq!(pulled(2).title(), "Pulled 2 commits from main");
        let found = |count| TimelineStep::ReviewFindingsReady { count };
        assert_eq!(found(1).title(), "Review found 1 finding");
        assert_eq!(found(3).title(), "Review found 3 findings");
    }
}
