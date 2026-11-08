//! A Thread taking a turn: starting its engine and a turn, stopping, pausing and resuming it,
//! swapping the session under it, and settling it when the engine says the turn is over.
//! Everything that moves a thread's status comes through here and `Workspaces::set_thread_status`,
//! so a thread only reads as running once the engine has its prompt, and every prompt an engine
//! gets is sent from here.
//!
//! Creating a thread's row (`provisioning`) and setup handing it its first turn are the
//! exceptions: setup owns the workspace until then.
//!
//! Pausing interrupts the turn and holds the thread: what is sent meanwhile queues, and resuming
//! sends the queue, or tells the agent to continue. Once a turn has settled, `TurnEnded` goes out
//! on the bus for the features that react to it.

pub mod commands;

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::sync::broadcast::error::RecvError;

use crate::attachments::PromptAttachment;
use crate::bus::DomainEvent;
use crate::engines::prompt::Prompt;
use crate::error::{Error, Result};
use crate::model::ThreadStatus;
use crate::state::AppState;
use crate::store::types::ThreadRow;
use crate::timeline::TimelineStep;

/// What a resumed thread is told when nothing queued while it was paused.
const RESUME_PROMPT: &str = "continue";

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ResumeThreadSessionInput {
    pub thread_id: String,
    /// One of the thread's conversations; see [`crate::transcript::ThreadConversations`].
    pub conversation_id: String,
}

/// Prompts sent to paused threads, waiting for their resume. In memory: a restart ends every
/// turn, pauses included.
#[derive(Default)]
pub struct PausedQueues(Mutex<HashMap<String, Vec<Prompt>>>);

impl PausedQueues {
    fn push(&self, thread_id: &str, prompt: Prompt) -> Result<()> {
        self.0
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?
            .entry(thread_id.to_string())
            .or_default()
            .push(prompt);
        Ok(())
    }

    fn take(&self, thread_id: &str) -> Vec<Prompt> {
        self.0
            .lock()
            .ok()
            .and_then(|mut queues| queues.remove(thread_id))
            .unwrap_or_default()
    }

    /// Puts back prompts that couldn't be sent, ahead of any queued since.
    fn restore(&self, thread_id: &str, mut prompts: Vec<Prompt>) {
        if let Ok(mut queues) = self.0.lock() {
            let queue = queues.entry(thread_id.to_string()).or_default();
            prompts.append(queue);
            *queue = prompts;
        }
    }
}

pub fn thread_row(state: &AppState, thread_id: &str) -> Result<ThreadRow> {
    state
        .store
        .thread_by_id(thread_id)?
        .ok_or_else(|| Error::Invalid(format!("unknown thread {thread_id}")))
}

/// A message as the thread shows it, where the transcript saved it.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SentMessage {
    pub seq: i64,
    /// The text with any attachments named after it.
    pub text: String,
}

/// A message into a thread, from the user or another thread: `start`, once the attachments are
/// read. An empty message is ignored.
pub async fn submit(
    state: &AppState,
    thread_id: &str,
    text: String,
    attachments: Vec<PromptAttachment>,
) -> Result<Option<SentMessage>> {
    let text = text.trim().to_string();
    if text.is_empty() && attachments.is_empty() {
        return Ok(None);
    }
    refuse_retired(&thread_row(state, thread_id)?)?;
    let folder = state.data_dir.attachments();
    // Reads and encodes the attached files, so it stays off the async workers.
    let (prompt, text) = tokio::task::spawn_blocking(move || {
        crate::attachments::build_prompt(&folder, &text, &attachments)
    })
    .await
    .map_err(|error| Error::Workspace(error.to_string()))??;
    let seq = start(state, &thread_row(state, thread_id)?, &text, prompt).await?;
    Ok(Some(SentMessage { seq, text }))
}

/// Starts a turn: `shown` goes in the thread for the user to read, `prompt` to the engine. The
/// engine is started if it isn't running. A paused thread queues the prompt for its resume and
/// stays paused. If this fails the thread keeps its status, though `shown` stays in the thread.
/// Returns where `shown` was saved.
pub async fn start(
    state: &AppState,
    thread: &ThreadRow,
    shown: &str,
    prompt: impl Into<Prompt>,
) -> Result<i64> {
    // First, so the thread shows the message while an engine takes its time to start.
    let seq = state.transcript.record_user_message(&thread.id, shown)?;
    if thread.status == ThreadStatus::Paused {
        state.paused.push(&thread.id, prompt.into())?;
    } else {
        send(state, thread, prompt.into()).await?;
    }
    state.out.changed();
    Ok(seq)
}

/// A failed send leaves the status alone: nothing would ever end a turn marked running.
async fn send(state: &AppState, thread: &ThreadRow, prompt: Prompt) -> Result<()> {
    state
        .engines
        .ensure(&thread.id, &thread.workspace_id)
        .await?;
    state.engines.prompt(&thread.id, prompt)?;
    record(state, thread, ThreadStatus::Running).await
}

/// A workspace's first turn, once its setup is done: a fresh engine, then the prompt. Setup sets
/// the statuses that go with it.
pub async fn begin(
    state: &AppState,
    thread_id: &str,
    workspace_id: &str,
    prompt: impl Into<Prompt>,
) -> Result<()> {
    state.engines.start(thread_id, workspace_id).await?;
    let prompt = prompt.into();
    if prompt.text.is_empty() && prompt.images.is_empty() {
        return Ok(());
    }
    state.engines.prompt(thread_id, prompt)
}

/// The thread reads as stopped at once, whatever the engine is still finishing: the step saying
/// who stopped it goes in the transcript now, and the engine reports the turn's end itself once
/// it has cancelled. Stopping ends a pause too, and drops whatever queued behind it.
pub async fn stop(state: &AppState, thread_id: &str) -> Result<()> {
    let thread = thread_row(state, thread_id)?;
    if state.engines.has_thread(thread_id)
        && let Err(error) = state.engines.cancel(thread_id)
    {
        log::warn!("cancelling {thread_id}'s turn: {error}");
    }
    state.paused.take(thread_id);
    state
        .transcript
        .record_step(thread_id, TimelineStep::AgentStopped)?;
    record(state, &thread, ThreadStatus::Idle).await?;
    state.out.changed();
    Ok(())
}

pub async fn pause(state: &AppState, thread_id: &str) -> Result<()> {
    let thread = thread_row(state, thread_id)?;
    if thread.status != ThreadStatus::Running {
        return Ok(());
    }
    // An engine that's gone has no turn left to interrupt; the thread still holds.
    if let Err(error) = state.engines.cancel(thread_id) {
        log::warn!("pausing {thread_id}: {error}");
    }
    state
        .transcript
        .record_step(thread_id, TimelineStep::AgentPaused)?;
    record(state, &thread, ThreadStatus::Paused).await?;
    state.out.changed();
    Ok(())
}

/// Sends what queued while the thread was paused, or tells it to continue. If the engine can't
/// take it the thread stays paused, with its queue.
pub async fn resume(state: &AppState, thread_id: &str) -> Result<()> {
    let thread = thread_row(state, thread_id)?;
    if thread.status != ThreadStatus::Paused {
        return Ok(());
    }
    let queued = state.paused.take(thread_id);
    let prompt = if queued.is_empty() {
        RESUME_PROMPT.into()
    } else {
        Prompt::merge(queued.clone())
    };
    if let Err(error) = send(state, &thread, prompt).await {
        state.paused.restore(thread_id, queued);
        return Err(error);
    }
    state
        .transcript
        .record_step(thread_id, TimelineStep::AgentResumed)?;
    state.out.changed();
    Ok(())
}

/// The step records the session it ended, so the conversation can be resumed later.
pub async fn new_session(state: &AppState, thread_id: &str) -> Result<()> {
    let thread = thread_row(state, thread_id)?;
    refuse_retired(&thread)?;
    let ended = state.store.thread_session(thread_id)?;
    state.engines.discard(thread_id)?;
    state.paused.take(thread_id);
    state.store.clear_thread_session(thread_id)?;
    state.transcript.record_step(
        thread_id,
        TimelineStep::SessionStarted {
            ended_session: ended,
        },
    )?;
    record(state, &thread, ThreadStatus::Idle).await?;
    state.out.changed();
    Ok(())
}

pub async fn resume_session(state: &AppState, input: ResumeThreadSessionInput) -> Result<()> {
    let thread_id = input.thread_id;
    let thread = thread_row(state, &thread_id)?;
    refuse_retired(&thread)?;
    let session = state
        .transcript
        .session_to_resume(&thread_id, &input.conversation_id)?;
    let ended = state.store.thread_session(&thread_id)?;
    state.engines.discard(&thread_id)?;
    state.paused.take(&thread_id);
    match session.as_deref() {
        Some(session) => state.store.set_thread_session(&thread_id, session)?,
        None => state.store.clear_thread_session(&thread_id)?,
    }
    state.transcript.record_step(
        &thread_id,
        TimelineStep::SessionResumed {
            conversation: input.conversation_id,
            ended_session: ended,
        },
    )?;
    record(state, &thread, ThreadStatus::Idle).await?;
    state.out.changed();
    Ok(())
}

/// Settles a thread whose engine ended its turn or exited: idle, unless it's paused (pausing
/// interrupts the engine, which answers with a turn end of its own). Then the features that
/// react to a turn ending hear of it, reading the thread as it now is.
pub async fn ended(state: &AppState, thread_id: &str) {
    match state.store.thread_by_id(thread_id) {
        Ok(Some(thread)) if thread.status != ThreadStatus::Paused => {
            if let Err(error) = record(state, &thread, ThreadStatus::Idle).await {
                log::warn!("settling thread {thread_id}: {error}");
            }
        }
        Ok(_) => {}
        Err(error) => log::warn!("reading thread {thread_id}: {error}"),
    }
    state.out.changed();
    state.bus.publish(DomainEvent::TurnEnded {
        thread_id: thread_id.to_string(),
    });
}

/// Runs `ended` for each turn the engines report, one at a time, for the life of the app.
pub fn start_settling(state: &AppState) {
    let state = state.clone();
    let mut turn_ends = state.engines.turn_ends();
    tauri::async_runtime::spawn(async move {
        loop {
            match turn_ends.recv().await {
                Ok(thread_id) => ended(&state, &thread_id).await,
                Err(RecvError::Lagged(missed)) => {
                    log::warn!("missed {missed} turn ends");
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
}

/// A retired thread (one a workspace outgrew, or a scratch's) stays readable but takes nothing.
fn refuse_retired(thread: &ThreadRow) -> Result<()> {
    if thread.retired {
        return Err(Error::Workspace(
            "this thread is read-only; start a new thread to carry on".into(),
        ));
    }
    Ok(())
}

async fn record(state: &AppState, thread: &ThreadRow, status: ThreadStatus) -> Result<()> {
    state
        .workspaces
        .set_thread_status(&thread.id, &thread.workspace_id, status)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::EngineCommand;
    use crate::thread_runtime::fake::FakeEngines;
    use std::sync::Arc;

    use crate::feedback::RecordingOutput;
    use crate::model::WorkspaceLifecycle;
    use crate::store::fixtures;
    use crate::store::types::WorkspaceRow;
    use tokio::sync::mpsc::UnboundedReceiver;

    type Engine = UnboundedReceiver<EngineCommand>;

    const T: &str = "t1";
    const W: &str = "w1";

    fn thread(id: &str, status: ThreadStatus, parent: Option<&str>) -> ThreadRow {
        ThreadRow {
            parent_thread_id: parent.map(str::to_string),
            ..fixtures::thread(id, W, status)
        }
    }

    /// A workspace with one thread in `status` and a live engine, whose commands are the
    /// receiver returned. `workspace` is the workspace's status to start from.
    async fn seeded(
        status: ThreadStatus,
        workspace: WorkspaceLifecycle,
    ) -> (AppState, Engine, Arc<RecordingOutput>) {
        let (state, recording, fake) = faked(status, workspace).await;
        let engine = fake.engine(T);
        state.engines.ensure(T, W).await.unwrap();
        (state, engine, recording)
    }

    /// Like `seeded`, with the engines themselves.
    async fn faked(
        status: ThreadStatus,
        workspace: WorkspaceLifecycle,
    ) -> (AppState, Arc<RecordingOutput>, FakeEngines) {
        let (state, recording, fake) =
            AppState::faked(crate::paths::DataDir::new(std::env::temp_dir()));
        state.store.upsert_repo(&fixtures::repo("r1")).unwrap();
        state
            .store
            .insert_workspace(&WorkspaceRow {
                worktree_path: std::env::temp_dir().display().to_string(),
                ..fixtures::workspace(W, workspace)
            })
            .unwrap();
        state.store.upsert_thread(&thread(T, status, None)).unwrap();
        (state, recording, fake)
    }

    fn thread_status(state: &AppState, id: &str) -> &'static str {
        state
            .store
            .thread_by_id(id)
            .unwrap()
            .unwrap()
            .status
            .as_str()
    }

    fn stored_workspace(state: &AppState) -> &'static str {
        state
            .store
            .workspace_by_id(W)
            .unwrap()
            .unwrap()
            .status
            .as_str()
    }

    async fn live_workspace(state: &AppState) -> WorkspaceLifecycle {
        state.workspaces.get(W).await.unwrap().status
    }

    fn user_messages(state: &AppState, id: &str) -> usize {
        state
            .transcript
            .page(id, 0, 100, usize::MAX)
            .unwrap()
            .entries
            .iter()
            .filter(|entry| entry.speaker.as_str() == "user")
            .count()
    }

    fn prompts(engine: &mut Engine) -> Vec<String> {
        let mut seen = Vec::new();
        while let Ok(command) = engine.try_recv() {
            if let EngineCommand::Prompt(prompt) = command {
                seen.push(prompt.text);
            }
        }
        seen
    }

    fn cancelled(engine: &mut Engine) -> bool {
        std::iter::from_fn(|| engine.try_recv().ok())
            .any(|command| matches!(command, EngineCommand::Cancel))
    }

    fn workspace_notified(recording: &RecordingOutput) -> bool {
        !recording.state_changes().is_empty()
    }

    #[tokio::test]
    async fn starting_a_turn_moves_the_thread_and_both_workspace_statuses_or_holds_if_paused() {
        use ThreadStatus::{Idle, Paused, Provisioning};
        use WorkspaceLifecycle as Lifecycle;
        for (status, sent, thread_after, stored_after, live_after) in [
            (Idle, true, "running", "running", Lifecycle::Running),
            (Provisioning, true, "running", "running", Lifecycle::Running),
            (Paused, false, "paused", "idle", Lifecycle::Idle),
        ] {
            let (state, mut engine, recording) = seeded(status, Lifecycle::Idle).await;

            let row = thread_row(&state, T).unwrap();
            start(&state, &row, "shown", "prompt").await.unwrap();

            assert_eq!(prompts(&mut engine).len(), usize::from(sent), "{status:?}");
            assert_eq!(thread_status(&state, T), thread_after, "{status:?}");
            assert_eq!(stored_workspace(&state), stored_after, "{status:?}");
            assert_eq!(live_workspace(&state).await, live_after, "{status:?}");
            assert_eq!(user_messages(&state, T), 1);
            assert!(workspace_notified(&recording));
        }
    }

    #[tokio::test]
    async fn a_failed_send_leaves_every_status_where_it_was() {
        let (state, recording, fake) = faked(ThreadStatus::Idle, WorkspaceLifecycle::Idle).await;
        fake.refuse_sends(T);

        let row = thread_row(&state, T).unwrap();
        let sent = start(&state, &row, "shown", "prompt").await;

        assert!(sent.is_err());
        assert_eq!(thread_status(&state, T), "idle");
        assert_eq!(stored_workspace(&state), "idle");
        assert_eq!(live_workspace(&state).await, WorkspaceLifecycle::Idle);
        assert!(!workspace_notified(&recording));
    }

    #[tokio::test]
    async fn a_prompt_queued_while_paused_goes_out_on_resume() {
        let (state, mut engine, _recording) =
            seeded(ThreadStatus::Running, WorkspaceLifecycle::Running).await;

        pause(&state, T).await.unwrap();
        assert!(cancelled(&mut engine));
        assert_eq!(thread_status(&state, T), "paused");
        assert_eq!(stored_workspace(&state), "paused");
        assert_eq!(live_workspace(&state).await, WorkspaceLifecycle::Paused);

        submit(&state, T, "while paused".into(), Vec::new())
            .await
            .unwrap();
        assert_eq!(thread_status(&state, T), "paused");
        assert!(prompts(&mut engine).is_empty());

        resume(&state, T).await.unwrap();
        assert_eq!(thread_status(&state, T), "running");
        assert_eq!(stored_workspace(&state), "running");
        assert_eq!(live_workspace(&state).await, WorkspaceLifecycle::Running);
        assert_eq!(prompts(&mut engine), ["while paused"]);
    }

    #[tokio::test]
    async fn pause_and_resume_only_apply_to_a_thread_that_can_take_them() {
        let (state, mut engine, recording) =
            seeded(ThreadStatus::Idle, WorkspaceLifecycle::Idle).await;

        pause(&state, T).await.unwrap();
        resume(&state, T).await.unwrap();

        assert_eq!(thread_status(&state, T), "idle");
        assert!(prompts(&mut engine).is_empty());
        assert!(recording.state_changes().is_empty());
    }

    #[tokio::test]
    async fn a_retired_thread_refuses_messages_and_sessions_and_an_empty_one_is_ignored() {
        let (state, mut engine, recording) =
            seeded(ThreadStatus::Idle, WorkspaceLifecycle::Idle).await;

        submit(&state, T, "   ".into(), Vec::new()).await.unwrap();
        assert!(recording.state_changes().is_empty());

        let retired = ThreadRow {
            retired: true,
            ..thread(T, ThreadStatus::Idle, None)
        };
        state.store.upsert_thread(&retired).unwrap();
        let refused = submit(&state, T, "hello".into(), Vec::new()).await;

        assert!(refused.is_err());
        assert!(new_session(&state, T).await.is_err());
        let resume = ResumeThreadSessionInput {
            thread_id: T.into(),
            conversation_id: "first".into(),
        };
        assert!(resume_session(&state, resume).await.is_err());
        assert!(prompts(&mut engine).is_empty());
        assert_eq!(thread_status(&state, T), "idle");
    }

    #[tokio::test]
    async fn stopping_idles_the_thread_at_once_and_cancels_the_engine() {
        let (state, mut engine, recording) =
            seeded(ThreadStatus::Running, WorkspaceLifecycle::Running).await;

        stop(&state, T).await.unwrap();

        assert!(cancelled(&mut engine));
        assert_eq!(thread_status(&state, T), "idle");
        assert_eq!(stored_workspace(&state), "idle");
        assert_eq!(live_workspace(&state).await, WorkspaceLifecycle::Idle);
        assert!(workspace_notified(&recording));
    }

    #[tokio::test]
    async fn a_sent_message_is_saved_streamed_and_returned_as_the_thread_shows_it() {
        let (state, _engine, _recording) =
            seeded(ThreadStatus::Idle, WorkspaceLifecycle::Idle).await;
        let mut stream = state.transcript.subscribe(T);
        let pasted = PromptAttachment {
            name: "notes.txt".into(),
            path: None,
            data_base64: Some("aGk=".into()),
        };

        let sent = submit(&state, T, "Look".into(), vec![pasted])
            .await
            .unwrap()
            .unwrap();

        assert_eq!(sent.text, "Look\n\nAttached: notes.txt");
        let streamed = stream.recv().await.unwrap();
        assert_eq!(streamed.seq, Some(sent.seq));
        assert_eq!(
            streamed.event,
            crate::transcript::TranscriptEvent::user_message(&sent.text)
        );
    }

    #[tokio::test]
    async fn stopping_cancels_the_engine_and_records_who_stopped_it_once() {
        let (state, mut engine, _recording) =
            seeded(ThreadStatus::Running, WorkspaceLifecycle::Running).await;
        let mut stream = state.transcript.subscribe(T);

        stop(&state, T).await.unwrap();

        assert!(matches!(engine.try_recv(), Ok(EngineCommand::Cancel)));
        let step = stream.recv().await.unwrap();
        assert!(step.seq.is_some());
        assert!(matches!(
            step.event,
            crate::transcript::TranscriptEvent::Step(crate::transcript::StepEvent::Step {
                step: TimelineStep::AgentStopped,
                ..
            })
        ));
        assert!(
            stream.try_recv().is_err(),
            "the turn's end is the engine's to report"
        );
        assert_eq!(thread_status(&state, T), "idle");
    }

    #[tokio::test]
    async fn a_new_or_resumed_session_drops_the_engine_and_idles_the_thread() {
        for resume in [false, true] {
            let (state, _engine, _recording) =
                seeded(ThreadStatus::Running, WorkspaceLifecycle::Running).await;
            state.store.set_thread_session(T, "opening").unwrap();
            new_session(&state, T).await.unwrap();
            assert_eq!(state.store.thread_session(T).unwrap(), None);

            if resume {
                state.store.set_thread_session(T, "later").unwrap();
                let input = ResumeThreadSessionInput {
                    thread_id: T.into(),
                    conversation_id: "first".into(),
                };
                resume_session(&state, input).await.unwrap();
                assert_eq!(
                    state.store.thread_session(T).unwrap().as_deref(),
                    Some("opening"),
                    "the core picks the session the conversation ended with"
                );
                assert_eq!(state.transcript.conversations(T).unwrap().current, "first");
            }

            assert!(!state.engines.has_thread(T));
            assert_eq!(thread_status(&state, T), "idle");
            assert_eq!(stored_workspace(&state), "idle");
            assert_eq!(live_workspace(&state).await, WorkspaceLifecycle::Idle);
        }
    }

    #[tokio::test]
    async fn resuming_a_conversation_the_thread_does_not_have_is_refused() {
        let (state, _engine, _recording) =
            seeded(ThreadStatus::Idle, WorkspaceLifecycle::Idle).await;
        state.store.set_thread_session(T, "current").unwrap();
        let input = ResumeThreadSessionInput {
            thread_id: T.into(),
            conversation_id: "after-99".into(),
        };
        assert!(resume_session(&state, input).await.is_err());
        assert_eq!(
            state.store.thread_session(T).unwrap().as_deref(),
            Some("current")
        );
    }

    #[tokio::test]
    async fn a_turn_ending_idles_syncs_notifies_and_is_published_after() {
        let (state, _engine, recording) =
            seeded(ThreadStatus::Running, WorkspaceLifecycle::Idle).await;
        state
            .workspaces
            .set_thread_status(T, W, ThreadStatus::Running)
            .await
            .unwrap();
        let mut events = state.bus.subscribe();

        ended(&state, T).await;

        assert_eq!(thread_status(&state, T), "idle");
        assert_eq!(stored_workspace(&state), "idle");
        assert_eq!(live_workspace(&state).await, WorkspaceLifecycle::Idle);
        assert!(workspace_notified(&recording));
        assert_eq!(
            events.try_recv().unwrap(),
            DomainEvent::TurnEnded {
                thread_id: T.into()
            }
        );
    }

    #[tokio::test]
    async fn a_paused_threads_own_turn_end_leaves_it_paused() {
        let (state, _engine, _recording) =
            seeded(ThreadStatus::Running, WorkspaceLifecycle::Running).await;
        pause(&state, T).await.unwrap();

        ended(&state, T).await;

        assert_eq!(thread_status(&state, T), "paused");
    }

    #[tokio::test]
    async fn a_sibling_still_running_keeps_the_workspace_running_after_a_turn_ends() {
        let (state, _engine, _recording) =
            seeded(ThreadStatus::Running, WorkspaceLifecycle::Idle).await;
        state
            .store
            .upsert_thread(&thread("t2", ThreadStatus::Running, None))
            .unwrap();
        state
            .workspaces
            .set_thread_status("t2", W, ThreadStatus::Running)
            .await
            .unwrap();

        ended(&state, T).await;

        assert_eq!(thread_status(&state, T), "idle");
        assert_eq!(stored_workspace(&state), "running");
        assert_eq!(live_workspace(&state).await, WorkspaceLifecycle::Running);
    }

    #[tokio::test]
    async fn a_resume_the_engine_refuses_keeps_the_thread_paused_with_its_queue() {
        let (state, recording, fake) = faked(ThreadStatus::Paused, WorkspaceLifecycle::Idle).await;
        submit(&state, T, "while paused".into(), Vec::new())
            .await
            .unwrap();
        fake.refuse_sends(T);

        assert!(resume(&state, T).await.is_err());
        assert_eq!(thread_status(&state, T), "paused");
        assert_eq!(stored_workspace(&state), "idle");
        assert!(!recording.state_changes().is_empty(), "the message showed");

        let mut engine = fake.engine(T);
        fake.accept_sends(T);
        resume(&state, T).await.unwrap();
        assert_eq!(prompts(&mut engine), ["while paused"]);
        assert_eq!(thread_status(&state, T), "running");
    }

    #[tokio::test]
    async fn a_resume_with_nothing_queued_tells_the_agent_to_continue_even_on_a_new_engine() {
        let (state, mut engine, _recording) =
            seeded(ThreadStatus::Running, WorkspaceLifecycle::Running).await;
        pause(&state, T).await.unwrap();
        state.engines.discard(T).unwrap();

        resume(&state, T).await.unwrap();

        assert_eq!(prompts(&mut engine), [RESUME_PROMPT]);
        assert_eq!(thread_status(&state, T), "running");
    }

    #[tokio::test]
    async fn stopping_drops_the_queue_and_a_thread_with_no_engine_still_stops() {
        let (state, recording, fake) = faked(ThreadStatus::Paused, WorkspaceLifecycle::Idle).await;
        submit(&state, T, "while paused".into(), Vec::new())
            .await
            .unwrap();

        stop(&state, T).await.unwrap();

        assert_eq!(thread_status(&state, T), "idle");
        assert!(workspace_notified(&recording));
        let mut engine = fake.engine(T);
        submit(&state, T, "next".into(), Vec::new()).await.unwrap();
        assert_eq!(
            prompts(&mut engine),
            ["next"],
            "the queue went with the stop"
        );
    }

    #[tokio::test]
    async fn stopping_a_thread_that_cannot_be_settled_says_so() {
        let (state, _recording, _fake) =
            faked(ThreadStatus::Running, WorkspaceLifecycle::Running).await;

        assert!(stop(&state, "no-such-thread").await.is_err());
    }
}
