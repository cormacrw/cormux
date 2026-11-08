//! What the core itself did in a thread, as the Timeline shows it. Every step the core writes goes
//! through [`record_step`], typed by what happened, so the UI picks a step's icon and chips from
//! its kind (`src/lib/thread/timeline-step.ts`) and rewording a title here can't change either.
//! The title and detail are stored alongside as display text.
//!
//! Rows written before this module carry no kind; `src/lib/thread/legacy-steps.ts` reads those.

use serde::{Deserialize, Serialize};
use specta::Type;
use uuid::Uuid;

use crate::composer::persist_agent_event;
use crate::engines::AgentEvent;
use crate::error::Result;
use crate::store::Store;

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

/// Something the core did in a thread. New kinds need a row in the UI's `STEP_PRESENTATION`.
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
    FindingsReceived {
        count: u32,
        titles: Vec<String>,
    },
    FindingsFixed {
        count: u32,
        files: Vec<String>,
    },
}

fn plural(count: u32) -> &'static str {
    if count == 1 { "" } else { "s" }
}

impl TimelineStep {
    /// The heading shown for this step. Display text only: nothing may read meaning from it.
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
            Self::FindingsReceived { count, .. } => {
                format!("Received {count} findings from the review")
            }
            Self::FindingsFixed { count, .. } => format!("Fixed {count} findings"),
        }
    }

    /// The line under the heading, for the steps that have one.
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
            | Self::SuggestionsForwarded => None,
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
    fn id(&self) -> String {
        match self {
            Self::PrOpened { number, .. } => format!("pr-open-{number}"),
            Self::SuggestionsSent { approval_id, .. } => format!("review-{approval_id}"),
            _ => format!("step-{}", Uuid::new_v4()),
        }
    }

    fn into_event(self) -> AgentEvent {
        AgentEvent::Step {
            id: self.id(),
            title: self.title(),
            detail: self.detail(),
            step: self,
        }
    }
}

/// Saves a step to the thread. The step shows the next time the UI reads the thread, so callers
/// raise whichever state change their action already does.
pub fn record_step(store: &Store, thread_id: &str, step: TimelineStep) -> Result<()> {
    persist_agent_event(store, thread_id, &step.into_event())
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;
    use crate::store::types::ThreadRow;

    fn store() -> Store {
        let store = Store::in_memory().unwrap();
        store
            .upsert_thread(&ThreadRow {
                id: "t1".into(),
                workspace_id: "w1".into(),
                title: "Lead".into(),
                role: Some(crate::model::ThreadRole::Lead),
                engine: "claude".into(),
                session_id: None,
                status: "idle".into(),
                used_tokens: None,
                context_size: None,
                cost_usd: None,
                transcript_readonly: false,
                retired: false,
                parent_thread_id: None,
            })
            .unwrap();
        store
    }

    fn rows(store: &Store) -> Vec<(String, Value)> {
        store
            .thread_events_after("t1", 0, 100)
            .unwrap()
            .into_iter()
            .map(|(_, kind, payload)| (kind, serde_json::from_str(&payload).unwrap()))
            .collect()
    }

    #[test]
    fn a_step_serializes_with_its_kind_and_display_text() {
        let event = TimelineStep::PrOpened {
            number: 12,
            branch: "feat/a".into(),
            base: "main".into(),
        }
        .into_event();
        assert_eq!(
            serde_json::to_value(&event).unwrap(),
            json!({
                "type": "step",
                "id": "pr-open-12",
                "title": "Opened PR #12",
                "detail": "feat/a → main",
                "step": { "kind": "prOpened", "number": 12, "branch": "feat/a", "base": "main" },
            })
        );
    }

    #[test]
    fn a_stored_step_reads_back_as_the_same_event() {
        let step = TimelineStep::BranchSwitched {
            branch: "feat/b".into(),
            change: BranchChange::AddedToStack {
                on_top_of: "feat/a".into(),
            },
        };
        let event = step.into_event();
        let text = serde_json::to_string(&event).unwrap();
        assert!(text.contains(r#""change":{"kind":"addedToStack","on_top_of":"feat/a"}"#));
        assert_eq!(serde_json::from_str::<AgentEvent>(&text).unwrap(), event);
    }

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
    fn the_writer_saves_a_tool_row_the_transcript_can_read() {
        let store = store();
        record_step(&store, "t1", TimelineStep::AgentPaused).unwrap();
        let rows = rows(&store);
        assert_eq!(rows.len(), 1);
        let (kind, payload) = &rows[0];
        // `read_thread` shows a tool row by its top-level `title`.
        assert_eq!(kind, "tool");
        assert_eq!(payload["type"], "step");
        assert_eq!(payload["title"], "You paused the agent");
        assert_eq!(payload["step"], json!({ "kind": "agentPaused" }));
        assert_eq!(payload["detail"], Value::Null);
    }

    #[test]
    fn only_steps_written_twice_share_an_id() {
        let store = store();
        for _ in 0..2 {
            record_step(&store, "t1", TimelineStep::AgentStopped).unwrap();
            let sent = TimelineStep::SuggestionsSent {
                approval_id: "a1".into(),
                what: "src/a.rs".into(),
            };
            record_step(&store, "t1", sent).unwrap();
        }
        let ids: Vec<String> = rows(&store)
            .iter()
            .map(|(_, payload)| payload["id"].as_str().unwrap().to_string())
            .collect();
        assert_ne!(ids[0], ids[2]);
        assert_eq!(ids[1], "review-a1");
        assert_eq!(ids[3], "review-a1");
    }

    /// The words users see today, which the timeline's existing rows also carry.
    #[test]
    fn every_step_keeps_its_wording() {
        let cases: Vec<(TimelineStep, &str, Option<&str>)> = vec![
            (TimelineStep::AgentStopped, "You stopped the agent", None),
            (TimelineStep::AgentResumed, "You resumed the agent", None),
            (
                TimelineStep::SessionStarted {
                    ended_session: None,
                },
                "Started a new session",
                None,
            ),
            (
                TimelineStep::StartedByThread {
                    workspace: "Auth".into(),
                },
                "Started by Auth",
                None,
            ),
            (
                TimelineStep::MessageFromThread {
                    workspace: "Auth".into(),
                },
                "Message from Auth",
                None,
            ),
            (
                TimelineStep::ReportFromThread {
                    title: "Auth".into(),
                },
                "Report from Auth",
                None,
            ),
            (
                TimelineStep::FileDeleted {
                    path: "a.rs".into(),
                },
                "Deleted a.rs",
                None,
            ),
            (TimelineStep::MigrationApplied, "Applied migration", None),
            (
                TimelineStep::SuggestionsForwarded,
                "Sent suggestions to Lead",
                None,
            ),
            (
                TimelineStep::SuggestionsSent {
                    approval_id: "a".into(),
                    what: "a.rs".into(),
                },
                "Suggestions sent",
                Some("a.rs"),
            ),
            (
                TimelineStep::BranchSwitched {
                    branch: "b".into(),
                    change: BranchChange::CheckedOut {
                        previous: "a".into(),
                    },
                },
                "Switched to `b`",
                Some("Checked out b in this worktree, previously a"),
            ),
            (
                TimelineStep::BranchSwitched {
                    branch: "b".into(),
                    change: BranchChange::Created {
                        previous: "a".into(),
                    },
                },
                "Switched to `b`",
                Some("Created and checked out b, previously a"),
            ),
            (
                TimelineStep::BranchSwitched {
                    branch: "b".into(),
                    change: BranchChange::CheckedOutOutside {
                        previous: "a".into(),
                    },
                },
                "Switched to `b`",
                Some("Checked out b outside Cormux, previously a"),
            ),
            (
                TimelineStep::BranchSwitched {
                    branch: "b".into(),
                    change: BranchChange::StackPushed,
                },
                "Switched to `b`",
                Some("Pushed the stack"),
            ),
            (
                TimelineStep::CommitsPulled {
                    count: 1,
                    base: "main".into(),
                    branch: "b".into(),
                },
                "Pulled 1 commit from main",
                Some("Merged into b with no conflicts"),
            ),
            (
                TimelineStep::CommitsPulled {
                    count: 2,
                    base: "main".into(),
                    branch: "b".into(),
                },
                "Pulled 2 commits from main",
                Some("Merged into b with no conflicts"),
            ),
            (
                TimelineStep::BranchRebased {
                    branch: "b".into(),
                    base: "main".into(),
                    replayed_onto: 3,
                },
                "Rebased b onto main",
                Some("Replayed local commits on top of 3 new commits with no conflicts"),
            ),
            (
                TimelineStep::WorktreeCreated {
                    branch: "b".into(),
                    base: "main".into(),
                },
                "Created worktree",
                Some("b ← main"),
            ),
            (
                TimelineStep::WorktreeReady {
                    repo: "app".into(),
                    setup_commands: 2,
                },
                "Ran worktree setup",
                Some("2 commands from app settings, see the Output tab"),
            ),
            (
                TimelineStep::WorktreeReady {
                    repo: "app".into(),
                    setup_commands: 0,
                },
                "Worktree ready",
                Some("No setup commands configured for app"),
            ),
            (
                TimelineStep::PrCheckedOut {
                    number: 7,
                    head: "b".into(),
                    base: "main".into(),
                    files_changed: 4,
                },
                "Checked out #7",
                Some("b → main · 4 files changed"),
            ),
            (
                TimelineStep::ReviewSubmitted { number: 7 },
                "Submitted review on #7",
                Some("Posted to GitHub"),
            ),
            (
                TimelineStep::ReviewFindingsUnreadable {
                    reason: "no block".into(),
                },
                "Couldn’t read the review findings",
                Some("no block"),
            ),
            (
                TimelineStep::FindingsReceived {
                    count: 2,
                    titles: vec!["One".into(), "Two".into()],
                },
                "Received 2 findings from the review",
                Some("One · Two"),
            ),
            (
                TimelineStep::FindingsFixed {
                    count: 2,
                    files: vec!["a.rs".into(), "b.rs".into()],
                },
                "Fixed 2 findings",
                Some("a.rs, b.rs"),
            ),
        ];
        for (step, title, detail) in cases {
            assert_eq!(step.title(), title);
            assert_eq!(step.detail().as_deref(), detail);
        }
    }
}
