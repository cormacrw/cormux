//! A Thread taking a turn: starting its engine and a turn, stopping, pausing and resuming it,
//! swapping the session under it, and settling it when the engine says the turn is over.
//! Everything that moves a thread's status comes through here and `Workspaces::set_thread_status`,
//! so a thread only reads as running once the engine has its prompt, and every prompt an engine
//! gets is sent from here. These run one at a time per thread, the settling of a turn's end
//! included, so none of them acts on a status another is changing.
//!
//! Creating a thread's row (`provisioning`) and setup handing it its first turn are the
//! exceptions: setup owns the workspace until then, and a message sent meanwhile is refused.
//! Teardown stops engines and settles what it stopped through here too (`stop_engines`,
//! `settle_stopped`).
//!
//! A message for a running thread goes to its engine at once, which takes it into the turn it's
//! running or starts its next turn on it; the thread runs until the engine has nothing left to
//! start. Pausing interrupts the turn and holds the thread: what it's sent meanwhile is held until
//! it resumes (or it is told to continue if nothing was). Once a turn has settled, `TurnEnded`
//! goes out on the bus for the features that react to it. The end of a turn that was paused, or
//! of an engine that was shut down on purpose, settles nothing: the thread already moved on.

pub mod commands;

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use specta::Type;

use tokio::sync::broadcast::error::RecvError;

use crate::attachments::PromptAttachment;
use crate::bus::{DomainEvent, TurnSpan};
use crate::engines::prompt::Prompt;
use crate::error::{Error, Result};
use crate::model::{ThreadStatus, WorkspaceLifecycle};
use crate::state::AppState;
use crate::store::types::ThreadRow;
use crate::thread_runtime::{ThreadOp, TurnEnd};
use crate::timeline::TimelineStep;

/// What a resumed thread is told when nothing was held while it was paused.
const RESUME_PROMPT: &str = "continue";

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ResumeThreadSessionInput {
    pub thread_id: String,
    /// One of the thread's conversations; see [`crate::transcript::ThreadConversations`].
    pub conversation_id: String,
}

/// Each thread's turn as this module keeps it. In memory: a restart ends every turn, pauses
/// included.
#[derive(Default)]
pub struct Turns {
    held: Mutex<HashMap<String, Vec<Held>>>,
    /// Where each thread's current turn began: the seq of the message that started it, or where
    /// the turn before ended if the engine started this one on a message it already had.
    began: Mutex<HashMap<String, i64>>,
}

/// A message waiting for its paused thread to resume, with the seq it was saved at.
struct Held {
    seq: i64,
    prompt: Prompt,
}

impl Turns {
    fn hold(&self, thread_id: &str, held: Held) {
        self.held
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .entry(thread_id.to_string())
            .or_default()
            .push(held);
    }

    fn take(&self, thread_id: &str) -> Vec<Held> {
        self.held
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(thread_id)
            .unwrap_or_default()
    }

    /// Puts back messages that couldn't be sent, ahead of any held since.
    fn restore(&self, thread_id: &str, mut prompts: Vec<Held>) {
        let mut held = self.held.lock().unwrap_or_else(|error| error.into_inner());
        let queue = held.entry(thread_id.to_string()).or_default();
        prompts.append(queue);
        *queue = prompts;
    }

    fn set_began(&self, thread_id: &str, seq: i64) {
        self.began
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(thread_id.to_string(), seq);
    }

    fn began(&self, thread_id: &str) -> Option<i64> {
        self.began
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(thread_id)
            .copied()
    }
}

/// Where a paused thread began holding what it's sent: the messages saved after its pause. None
/// for a thread that isn't paused, which holds nothing. Holding is in memory, so after a quit
/// this is what the last run held and never sent.
pub fn holding_after(store: &crate::store::Store, thread: &ThreadRow) -> Result<Option<i64>> {
    if thread.status != ThreadStatus::Paused {
        return Ok(None);
    }
    store.last_step_seq(&thread.id, &TimelineStep::AgentPaused)
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
/// engine is started if it isn't running. A running thread's engine gets the prompt mid-turn; a
/// paused thread holds it for its resume and stays paused. If this fails the thread keeps its
/// status, though `shown` stays in the thread. Returns where `shown` was saved.
pub async fn start(
    state: &AppState,
    thread: &ThreadRow,
    shown: &str,
    prompt: impl Into<Prompt>,
) -> Result<i64> {
    let op = state.engines.operation(&thread.id).await;
    let thread = thread_row(state, &thread.id)?;
    refuse_during_setup(state, &thread).await?;
    // First, so the thread shows the message while an engine takes its time to start.
    let seq = state.transcript.record_user_message(&thread.id, shown)?;
    let prompt = prompt.into();
    if thread.status == ThreadStatus::Paused {
        state.turns.hold(&thread.id, Held { seq, prompt });
    } else {
        let mid_turn =
            thread.status == ThreadStatus::Running && state.engines.has_thread(&thread.id);
        send(state, &op, &thread, prompt).await?;
        if !mid_turn {
            state.turns.set_began(&thread.id, seq);
        }
    }
    state.out.changed();
    Ok(seq)
}

/// Sends `prompts` as one turn, which begins at the newest of them. On failure they come back.
async fn deliver(
    state: &AppState,
    op: &ThreadOp,
    thread: &ThreadRow,
    prompts: Vec<Held>,
) -> std::result::Result<(), (Error, Vec<Held>)> {
    let since = prompts.iter().map(|held| held.seq).max();
    let prompt = Prompt::merge(prompts.iter().map(|held| held.prompt.clone()).collect());
    if let Err(error) = send(state, op, thread, prompt).await {
        return Err((error, prompts));
    }
    if let Some(since) = since {
        state.turns.set_began(&thread.id, since);
    }
    Ok(())
}

/// A failed send leaves the status alone: nothing would ever end a turn marked running.
async fn send(state: &AppState, op: &ThreadOp, thread: &ThreadRow, prompt: Prompt) -> Result<()> {
    state.engines.ensure(op, &thread.workspace_id).await?;
    state.engines.prompt(op, prompt)?;
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
    let op = state.engines.operation(thread_id).await;
    state.engines.start(&op, workspace_id).await?;
    let prompt = prompt.into();
    if prompt.text.is_empty() && prompt.images.is_empty() {
        return Ok(());
    }
    state.engines.prompt(&op, prompt)?;
    state
        .turns
        .set_began(thread_id, state.transcript.last_message_seq(thread_id)?);
    Ok(())
}

/// The thread reads as stopped at once, whatever the engine is still finishing: the step saying
/// who stopped it goes in the transcript now, and the engine reports the turn's end itself once
/// it has cancelled. Stopping ends a pause too, and drops whatever was held for the thread.
pub async fn stop(state: &AppState, thread_id: &str) -> Result<()> {
    let op = state.engines.operation(thread_id).await;
    let thread = thread_row(state, thread_id)?;
    let mut engine_reports_end = false;
    if thread.status.is_unfinished() && state.engines.has_thread(thread_id) {
        match state.engines.cancel(&op) {
            Ok(()) => engine_reports_end = thread.status == ThreadStatus::Running,
            Err(error) => log::warn!("cancelling {thread_id}'s turn: {error}"),
        }
    }
    let dropped = state.turns.take(thread_id);
    state
        .transcript
        .record_step(thread_id, TimelineStep::AgentStopped)?;
    record(state, &thread, ThreadStatus::Idle).await?;
    state.out.changed();
    announce_dropped(state, thread_id, dropped);
    if thread.status.is_mid_turn() && !engine_reports_end {
        announce_stopped(state, thread_id)?;
    }
    Ok(())
}

/// Stops the threads' engines for good from outside their turns (teardown), each under its turn
/// lock so nothing starts, resumes or settles the thread while its engine goes down. Engines
/// stopped this way report no end, so nothing settles the threads: see [`settle_stopped`].
pub async fn stop_engines(state: &AppState, thread_ids: &[String]) -> Result<()> {
    let mut stops = tokio::task::JoinSet::new();
    for id in thread_ids {
        let (state, id) = (state.clone(), id.clone());
        stops.spawn(async move {
            let op = state.engines.operation(&id).await;
            state.engines.stop(&op).await
        });
    }
    for stopped in stops.join_all().await {
        stopped?;
    }
    Ok(())
}

/// Settles threads whose engines [`stop_engines`] stopped, once they carry on (a teardown that
/// failed): a running one is idle. A paused one stays paused with what it holds, since resuming
/// launches a new engine.
pub async fn settle_stopped(state: &AppState, thread_ids: &[String]) -> Result<()> {
    for id in thread_ids {
        let _op = state.engines.operation(id).await;
        let thread = thread_row(state, id)?;
        if thread.status == ThreadStatus::Running {
            record(state, &thread, ThreadStatus::Idle).await?;
        }
    }
    Ok(())
}

pub async fn pause(state: &AppState, thread_id: &str) -> Result<()> {
    let op = state.engines.operation(thread_id).await;
    let thread = thread_row(state, thread_id)?;
    if thread.status != ThreadStatus::Running {
        return Ok(());
    }
    // An engine that's gone has no turn left to interrupt; the thread still holds.
    if let Err(error) = state.engines.cancel(&op) {
        log::warn!("pausing {thread_id}: {error}");
    }
    state
        .transcript
        .record_step(thread_id, TimelineStep::AgentPaused)?;
    record(state, &thread, ThreadStatus::Paused).await?;
    state.out.changed();
    Ok(())
}

/// Sends what was held while the thread was paused, or tells it to continue. If the engine
/// can't take it the thread stays paused, with what was held.
pub async fn resume(state: &AppState, thread_id: &str) -> Result<()> {
    let op = state.engines.operation(thread_id).await;
    let thread = thread_row(state, thread_id)?;
    if thread.status != ThreadStatus::Paused {
        return Ok(());
    }
    let held = state.turns.take(thread_id);
    let sent = if held.is_empty() {
        send(state, &op, &thread, RESUME_PROMPT.into()).await
    } else {
        deliver(state, &op, &thread, held)
            .await
            .map_err(|(error, unsent)| {
                state.turns.restore(thread_id, unsent);
                error
            })
    };
    sent?;
    state
        .transcript
        .record_step(thread_id, TimelineStep::AgentResumed)?;
    state.out.changed();
    Ok(())
}

/// The step records the session it ended, so the conversation can be resumed later.
pub async fn new_session(state: &AppState, thread_id: &str) -> Result<()> {
    let op = state.engines.operation(thread_id).await;
    let thread = thread_row(state, thread_id)?;
    refuse_retired(&thread)?;
    let ended = state.store.thread_session(thread_id)?;
    state.engines.discard(&op)?;
    let dropped = state.turns.take(thread_id);
    state.store.clear_thread_session(thread_id)?;
    state.transcript.record_step(
        thread_id,
        TimelineStep::SessionStarted {
            ended_session: ended,
        },
    )?;
    record(state, &thread, ThreadStatus::Idle).await?;
    state.out.changed();
    settle_replaced(state, &thread, dropped)
}

pub async fn resume_session(state: &AppState, input: ResumeThreadSessionInput) -> Result<()> {
    let thread_id = input.thread_id;
    let op = state.engines.operation(&thread_id).await;
    let thread = thread_row(state, &thread_id)?;
    refuse_retired(&thread)?;
    let session = state
        .transcript
        .session_to_resume(&thread_id, &input.conversation_id)?;
    let ended = state.store.thread_session(&thread_id)?;
    state.engines.discard(&op)?;
    let dropped = state.turns.take(&thread_id);
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
    settle_replaced(state, &thread, dropped)
}

/// A replaced session's engine reports nothing, so a turn it was in the middle of ends here.
fn settle_replaced(state: &AppState, thread: &ThreadRow, dropped: Vec<Held>) -> Result<()> {
    announce_dropped(state, &thread.id, dropped);
    if thread.status.is_mid_turn() {
        announce_stopped(state, &thread.id)?;
    }
    Ok(())
}

fn announce_dropped(state: &AppState, thread_id: &str, dropped: Vec<Held>) {
    if dropped.is_empty() {
        return;
    }
    state.bus.publish(DomainEvent::PromptsDropped {
        thread_id: thread_id.to_string(),
        message_seqs: dropped.iter().map(|held| held.seq).collect(),
    });
}

fn announce_stopped(state: &AppState, thread_id: &str) -> Result<()> {
    let span = span(state, thread_id)?;
    state.bus.publish(DomainEvent::TurnEnded {
        thread_id: thread_id.to_string(),
        span,
        stopped: true,
    });
    Ok(())
}

fn span(state: &AppState, thread_id: &str) -> Result<TurnSpan> {
    Ok(TurnSpan {
        since: state.turns.began(thread_id),
        through: state.transcript.last_message_seq(thread_id)?,
    })
}

/// Settles a thread whose engine ended its turn or exited: idle, unless it's paused, or the
/// engine has a message to start its next turn on (one it hadn't taken, or one sent since this
/// end), which keeps the thread running from where this turn ended. Then the features that react
/// to a turn ending hear of it, reading the thread as it now is.
///
/// An end from an engine since replaced or shut down is dropped. So is an interrupted turn's,
/// unless it was stopped and nothing has started since: a paused turn isn't over, and a turn
/// started after a stop isn't the one that ended.
pub async fn ended(state: &AppState, end: &TurnEnd) {
    let thread_id = end.thread_id.as_str();
    let _op = state.engines.operation(thread_id).await;
    if !state.engines.is_current(thread_id, end.engine) {
        return;
    }
    let thread = match state.store.thread_by_id(thread_id) {
        Ok(Some(thread)) => thread,
        Ok(None) => return,
        Err(error) => {
            log::warn!("reading thread {thread_id}: {error}");
            return;
        }
    };
    if end.interrupted && thread.status.is_unfinished() {
        return;
    }
    let span = match span(state, thread_id) {
        Ok(span) => span,
        Err(error) => {
            log::warn!("reading where {thread_id}'s turn ended: {error}");
            return;
        }
    };
    if !end.interrupted && thread.status != ThreadStatus::Paused {
        if end.queued || state.engines.in_turn(thread_id) {
            state.turns.set_began(thread_id, span.through);
        } else if let Err(error) = record(state, &thread, ThreadStatus::Idle).await {
            log::warn!("settling thread {thread_id}: {error}");
        }
    }
    state.out.changed();
    state.bus.publish(DomainEvent::TurnEnded {
        thread_id: thread_id.to_string(),
        span,
        stopped: end.interrupted,
    });
}

/// The thread's current engine ending its turn, as a test reports it.
#[cfg(test)]
pub async fn end_turn(state: &AppState, thread_id: &str) {
    state.engines.close_turn(thread_id);
    let end = TurnEnd {
        thread_id: thread_id.to_string(),
        engine: state.engines.current_engine(thread_id),
        interrupted: false,
        queued: false,
    };
    ended(state, &end).await;
}

/// Runs `ended` for each turn the engines report, one at a time, for the life of the app.
pub fn start_settling(state: &AppState) {
    let state = state.clone();
    let mut turn_ends = state.engines.turn_ends();
    tauri::async_runtime::spawn(async move {
        loop {
            match turn_ends.recv().await {
                Ok(end) => ended(&state, &end).await,
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

/// Setup launches the thread's engine itself once it's done, and would replace one started now.
async fn refuse_during_setup(state: &AppState, thread: &ThreadRow) -> Result<()> {
    let setting_up = state
        .workspaces
        .get(&thread.workspace_id)
        .await
        .is_ok_and(|workspace| workspace.status == WorkspaceLifecycle::Provisioning);
    if setting_up {
        return Err(Error::Workspace(
            "This workspace is still being set up; send once setup is done.".into(),
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
        state
            .engines
            .ensure(&state.engines.operation(T).await, W)
            .await
            .unwrap();
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

    /// Teardown stopped engines and idled threads outside the per-thread lock, so a turn
    /// settling or starting meanwhile could act on a status teardown was changing.
    #[tokio::test(start_paused = true)]
    async fn teardown_stops_and_settles_a_thread_only_between_its_operations() {
        let (state, mut engine, _recording) =
            seeded(ThreadStatus::Running, WorkspaceLifecycle::Running).await;
        let ids = [T.to_string()];
        let operation = state.engines.operation(T).await;

        let stopping = tokio::spawn({
            let (state, ids) = (state.clone(), ids.clone());
            async move { stop_engines(&state, &ids).await }
        });
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        assert!(state.engines.has_thread(T), "stopped mid-operation");

        drop(operation);
        while let Some(command) = engine.recv().await {
            if matches!(command, EngineCommand::Shutdown) {
                break;
            }
        }
        drop(engine);
        stopping.await.unwrap().unwrap();
        assert!(!state.engines.has_thread(T));
        settle_stopped(&state, &ids).await.unwrap();
        assert_eq!(thread_status(&state, T), "idle");
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

        end_turn(&state, T).await;

        assert_eq!(thread_status(&state, T), "idle");
        assert_eq!(stored_workspace(&state), "idle");
        assert_eq!(live_workspace(&state).await, WorkspaceLifecycle::Idle);
        assert!(workspace_notified(&recording));
        assert_eq!(
            events.try_recv().unwrap(),
            DomainEvent::TurnEnded {
                thread_id: T.into(),
                span: TurnSpan {
                    since: None,
                    through: 0
                },
                stopped: false,
            }
        );
    }

    #[tokio::test]
    async fn a_paused_threads_own_turn_end_leaves_it_paused() {
        let (state, _engine, _recording) =
            seeded(ThreadStatus::Running, WorkspaceLifecycle::Running).await;
        pause(&state, T).await.unwrap();

        end_turn(&state, T).await;

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

        end_turn(&state, T).await;

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
        state
            .engines
            .discard(&state.engines.operation(T).await)
            .unwrap();

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

    /// An idle thread with a live engine, and the turn ends that engine reports.
    async fn live(
        state_status: ThreadStatus,
    ) -> (
        AppState,
        FakeEngines,
        tokio::sync::broadcast::Receiver<TurnEnd>,
    ) {
        let (state, _recording, fake) = faked(state_status, WorkspaceLifecycle::Idle).await;
        let _engine = fake.engine(T);
        state
            .engines
            .ensure(&state.engines.operation(T).await, W)
            .await
            .unwrap();
        let ends = state.engines.turn_ends();
        (state, fake, ends)
    }

    async fn report_end(
        state: &AppState,
        fake: &FakeEngines,
        ends: &mut tokio::sync::broadcast::Receiver<TurnEnd>,
    ) {
        let end = engine_ends(fake, ends, false).await;
        ended(state, &end).await;
    }

    /// The engine reporting its turn's end, `queued` if it has a message left to start the next
    /// on, as the runtime hands it on before it settles.
    async fn engine_ends(
        fake: &FakeEngines,
        ends: &mut tokio::sync::broadcast::Receiver<TurnEnd>,
        queued: bool,
    ) -> TurnEnd {
        fake.report(
            T,
            crate::engines::EngineOutput::TurnEnded {
                stop_reason: crate::model::StopReason::EndTurn,
                error: None,
                next_queued: queued,
            },
        );
        tokio::time::timeout(std::time::Duration::from_secs(5), ends.recv())
            .await
            .unwrap()
            .unwrap()
    }

    fn turn_ends(events: &mut tokio::sync::broadcast::Receiver<DomainEvent>) -> Vec<TurnSpan> {
        std::iter::from_fn(|| events.try_recv().ok())
            .filter_map(|event| match event {
                DomainEvent::TurnEnded { span, .. } => Some(span),
                _ => None,
            })
            .collect()
    }

    fn turn_ended(events: &mut tokio::sync::broadcast::Receiver<DomainEvent>) -> bool {
        std::iter::from_fn(|| events.try_recv().ok())
            .any(|event| matches!(event, DomainEvent::TurnEnded { .. }))
    }

    /// Bug: the paused turn's end arrived after Resume and idled the thread mid-turn; pausing
    /// alone also published `TurnEnded`, so a review read its half-written reply and a parent
    /// heard its child was idle.
    #[tokio::test]
    async fn a_paused_turns_late_end_neither_idles_the_resumed_thread_nor_reads_as_ended() {
        let (state, fake, mut ends) = live(ThreadStatus::Idle).await;
        let mut events = state.bus.subscribe();
        start(&state, &thread_row(&state, T).unwrap(), "go", "go")
            .await
            .unwrap();

        pause(&state, T).await.unwrap();
        resume(&state, T).await.unwrap();
        report_end(&state, &fake, &mut ends).await;
        assert_eq!(thread_status(&state, T), "running");
        assert!(!turn_ended(&mut events));

        report_end(&state, &fake, &mut ends).await;
        assert_eq!(thread_status(&state, T), "idle");
        assert!(turn_ended(&mut events));
    }

    #[tokio::test]
    async fn a_paused_turns_end_leaves_it_paused_and_unended() {
        let (state, fake, mut ends) = live(ThreadStatus::Idle).await;
        let mut events = state.bus.subscribe();
        start(&state, &thread_row(&state, T).unwrap(), "go", "go")
            .await
            .unwrap();

        pause(&state, T).await.unwrap();
        report_end(&state, &fake, &mut ends).await;

        assert_eq!(thread_status(&state, T), "paused");
        assert!(!turn_ended(&mut events));
    }

    /// Bug: the stopped turn's end idled the turn the next message started.
    #[tokio::test]
    async fn a_stopped_turns_end_leaves_the_next_turn_running() {
        let (state, fake, mut ends) = live(ThreadStatus::Idle).await;
        start(&state, &thread_row(&state, T).unwrap(), "go", "go")
            .await
            .unwrap();

        stop(&state, T).await.unwrap();
        start(&state, &thread_row(&state, T).unwrap(), "again", "again")
            .await
            .unwrap();
        report_end(&state, &fake, &mut ends).await;

        assert_eq!(thread_status(&state, T), "running");
    }

    #[tokio::test]
    async fn a_stopped_turns_end_still_tells_the_features_it_ended() {
        let (state, fake, mut ends) = live(ThreadStatus::Idle).await;
        let mut events = state.bus.subscribe();
        start(&state, &thread_row(&state, T).unwrap(), "go", "go")
            .await
            .unwrap();

        stop(&state, T).await.unwrap();
        report_end(&state, &fake, &mut ends).await;

        assert_eq!(thread_status(&state, T), "idle");
        assert!(turn_ended(&mut events));
    }

    #[tokio::test]
    async fn stopping_a_thread_that_cannot_be_settled_says_so() {
        let (state, _recording, _fake) =
            faked(ThreadStatus::Running, WorkspaceLifecycle::Running).await;

        assert!(stop(&state, "no-such-thread").await.is_err());
    }

    /// Bug: a message sent while the agent worked was held until its turn ended, so the user
    /// couldn't steer a running agent.
    #[tokio::test]
    async fn a_message_sent_mid_turn_reaches_the_engine_at_once() {
        let (state, _recording, fake) = faked(ThreadStatus::Idle, WorkspaceLifecycle::Idle).await;
        let mut engine = fake.engine(T);
        let mut stream = state.transcript.subscribe(T);
        start(&state, &thread_row(&state, T).unwrap(), "go", "go")
            .await
            .unwrap();
        assert_eq!(prompts(&mut engine), ["go"]);

        let later = start(&state, &thread_row(&state, T).unwrap(), "also", "also")
            .await
            .unwrap();

        assert_eq!(prompts(&mut engine), ["also"]);
        assert_eq!(thread_status(&state, T), "running");
        let streamed: Vec<_> = std::iter::from_fn(|| stream.try_recv().ok()).collect();
        assert_eq!(streamed.last().and_then(|event| event.seq), Some(later));
    }

    /// Bug: the engine answered the first turn and went on to a message sent meanwhile, but the
    /// first turn's end idled the thread, and a review read both turns as one.
    #[tokio::test]
    async fn a_turn_ending_with_a_message_still_queued_leaves_the_thread_running() {
        let (state, _recording, fake) = faked(ThreadStatus::Idle, WorkspaceLifecycle::Idle).await;
        let _engine = fake.engine(T);
        let mut ends = state.engines.turn_ends();
        let mut events = state.bus.subscribe();
        let first = start(&state, &thread_row(&state, T).unwrap(), "go", "go")
            .await
            .unwrap();
        start(&state, &thread_row(&state, T).unwrap(), "also", "also")
            .await
            .unwrap();

        let end = engine_ends(&fake, &mut ends, true).await;
        ended(&state, &end).await;
        assert_eq!(thread_status(&state, T), "running");
        let [span] = turn_ends(&mut events)[..] else {
            panic!("the first turn is over");
        };
        assert_eq!(span.since, Some(first));

        report_end(&state, &fake, &mut ends).await;
        assert_eq!(thread_status(&state, T), "idle");
        assert_eq!(
            turn_ends(&mut events),
            [TurnSpan {
                since: Some(span.through),
                through: span.through
            }],
            "the second turn began where the first ended"
        );
    }

    #[tokio::test]
    async fn a_message_taken_into_the_turn_or_sent_after_its_end_is_settled_right() {
        let (state, _recording, fake) = faked(ThreadStatus::Idle, WorkspaceLifecycle::Idle).await;
        let _engine = fake.engine(T);
        let mut ends = state.engines.turn_ends();
        start(&state, &thread_row(&state, T).unwrap(), "go", "go")
            .await
            .unwrap();
        start(&state, &thread_row(&state, T).unwrap(), "also", "also")
            .await
            .unwrap();
        report_end(&state, &fake, &mut ends).await;
        assert_eq!(thread_status(&state, T), "idle", "one turn answered both");

        start(&state, &thread_row(&state, T).unwrap(), "next", "next")
            .await
            .unwrap();
        let end = engine_ends(&fake, &mut ends, false).await;
        start(&state, &thread_row(&state, T).unwrap(), "then", "then")
            .await
            .unwrap();
        ended(&state, &end).await;
        assert_eq!(
            thread_status(&state, T),
            "running",
            "the engine has a message sent after the turn's end"
        );
    }

    /// Bug: only `ensure` took the launch lock and a message could start a turn during setup,
    /// so setup's own launch then shut the user's turn down mid-prompt.
    #[tokio::test]
    async fn a_message_during_setup_is_refused_and_launches_nothing() {
        let (state, _recording, fake) =
            faked(ThreadStatus::Provisioning, WorkspaceLifecycle::Provisioning).await;
        let mut engine = fake.engine(T);

        let refused = start(&state, &thread_row(&state, T).unwrap(), "hi", "hi").await;

        assert!(refused.unwrap_err().to_string().contains("set up"));
        assert!(!state.engines.has_thread(T));
        assert!(prompts(&mut engine).is_empty());
        assert_eq!(user_messages(&state, T), 0);
    }

    #[tokio::test]
    async fn stopping_or_replacing_a_paused_turn_ends_it_and_drops_what_was_held() {
        for replace in [false, true] {
            let (state, _engine, _recording) =
                seeded(ThreadStatus::Running, WorkspaceLifecycle::Running).await;
            pause(&state, T).await.unwrap();
            let held = start(&state, &thread_row(&state, T).unwrap(), "later", "later")
                .await
                .unwrap();
            let mut events = state.bus.subscribe();

            if replace {
                new_session(&state, T).await.unwrap();
            } else {
                stop(&state, T).await.unwrap();
            }

            let published: Vec<DomainEvent> =
                std::iter::from_fn(|| events.try_recv().ok()).collect();
            assert!(published.contains(&DomainEvent::PromptsDropped {
                thread_id: T.into(),
                message_seqs: vec![held],
            }));
            assert!(
                published
                    .iter()
                    .any(|event| matches!(event, DomainEvent::TurnEnded { stopped: true, .. })),
                "{published:?}"
            );
        }
    }
}
