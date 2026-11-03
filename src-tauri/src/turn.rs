//! A Thread taking a turn: starting one, stopping, pausing and resuming it, swapping the
//! session under it, and what follows when it ends. Everything that moves a thread's status
//! comes through here, so the status, the workspace's status and the live `WorkspaceManager`
//! record move together, and a thread only reads as running once the engine has its prompt.
//!
//! Creating a thread's row (`provisioning`) and setup handing it its first turn are the
//! exceptions: setup owns the workspace until then.

use tokio::sync::broadcast::error::RecvError;

use crate::attachments::PromptAttachment;
use crate::composer::persist_user_message;
use crate::engines::prompt::Prompt;
use crate::error::{Error, Result};
use crate::feedback::Output;
use crate::ipc::types::{ResumeThreadSessionInput, StateChangeKind};
use crate::state::AppState;
use crate::store::Store;
use crate::store::types::ThreadRow;
use crate::timeline::{TimelineStep, record_step};
use crate::workspace::ThreadStatus;

/// Any thread, a scratch's included.
pub fn thread_row(state: &AppState, thread_id: &str) -> Result<ThreadRow> {
    state
        .store
        .thread_by_id(thread_id)?
        .ok_or_else(|| Error::Store(format!("unknown thread {thread_id}")))
}

/// A message into a thread, from the user or another thread: `start`, once the attachments are
/// read. An empty message is ignored.
pub async fn submit(
    out: &Output,
    state: &AppState,
    thread_id: &str,
    text: String,
    attachments: Vec<PromptAttachment>,
) -> Result<()> {
    let text = text.trim().to_string();
    if text.is_empty() && attachments.is_empty() {
        return Ok(());
    }
    if state
        .store
        .thread_by_id(thread_id)?
        .is_some_and(|thread| thread.retired)
    {
        return Err(Error::Workspace(
            "this thread is read-only; start a new thread to carry on".into(),
        ));
    }
    let folder = state.data_dir.join("attachments");
    // Reads and encodes the attached files, so it stays off the async workers.
    let (prompt, text) = tokio::task::spawn_blocking(move || {
        crate::attachments::build_prompt(&folder, &text, &attachments)
    })
    .await
    .map_err(|error| Error::Workspace(error.to_string()))??;
    start(out, state, &thread_row(state, thread_id)?, &text, prompt).await
}

/// Starts a turn: `shown` goes in the thread for the user to read, `prompt` to the engine. The
/// engine is started if it isn't running. A paused thread queues the prompt for its resume and
/// stays paused. If this fails the thread keeps its status, though `shown` stays in the thread.
pub async fn start(
    out: &Output,
    state: &AppState,
    thread: &ThreadRow,
    shown: &str,
    prompt: impl Into<Prompt>,
) -> Result<()> {
    let held = thread.has_status(ThreadStatus::Paused);
    if !held {
        crate::provisioning::ensure_thread_engine(
            state,
            &thread.id,
            &thread.engine,
            &thread.workspace_id,
        )
        .await?;
    }
    persist_user_message(&state.store, &thread.id, shown)?;
    send(state, thread, prompt.into(), held).await?;
    notify(out, state);
    Ok(())
}

async fn send(state: &AppState, thread: &ThreadRow, prompt: Prompt, held: bool) -> Result<()> {
    // A failed send must not mark the thread running: nothing would ever end that turn.
    state.engines.submit_prompt(&thread.id, prompt, held)?;
    if !held {
        record(state, thread, ThreadStatus::Running).await?;
    }
    Ok(())
}

pub async fn stop(out: &Output, state: &AppState, thread_id: &str) {
    let thread = thread_row(state, thread_id).ok();
    let _ = state.engines.cancel(thread_id);
    let _ = record_step(&state.store, thread_id, TimelineStep::AgentStopped);
    // Stopping ends a pause too, and drops whatever queued behind it.
    let _ = state.engines.release_thread(thread_id);
    let _ = state.store.set_thread_status(thread_id, ThreadStatus::Idle);
    if let Some(thread) = thread {
        sync_manager(state, &thread, ThreadStatus::Idle).await;
    }
    notify(out, state);
}

/// Only a running thread can pause; any other is left as it is.
pub async fn pause(out: &Output, state: &AppState, thread_id: &str) -> Result<()> {
    let thread = thread_row(state, thread_id)?;
    if !thread.has_status(ThreadStatus::Running) {
        return Ok(());
    }
    state.engines.hold_thread(thread_id)?;
    record_step(&state.store, thread_id, TimelineStep::AgentPaused)?;
    record(state, &thread, ThreadStatus::Paused).await?;
    notify(out, state);
    Ok(())
}

/// Sends what queued while the thread was paused, or tells it to continue.
pub async fn resume(out: &Output, state: &AppState, thread_id: &str) -> Result<()> {
    let thread = thread_row(state, thread_id)?;
    if !thread.has_status(ThreadStatus::Paused) {
        return Ok(());
    }
    record_step(&state.store, thread_id, TimelineStep::AgentResumed)?;
    record(state, &thread, ThreadStatus::Running).await?;
    if let Some(prompt) = state.engines.release_thread(thread_id)? {
        let _ = state.engines.prompt(thread_id, prompt);
    }
    notify(out, state);
    Ok(())
}

/// The step records the session it ended, so the conversation can be resumed later.
pub async fn new_session(out: &Output, state: &AppState, thread_id: &str) -> Result<()> {
    let thread = thread_row(state, thread_id)?;
    if state.store.scratch_for_thread(thread_id)?.is_some() {
        return Err(Error::Workspace(
            "a scratch can't start a new session".into(),
        ));
    }
    let ended = state.store.thread_session(thread_id)?;
    state.engines.discard(thread_id)?;
    state.store.clear_thread_session(thread_id)?;
    record_step(
        &state.store,
        thread_id,
        TimelineStep::SessionStarted {
            ended_session: ended,
        },
    )?;
    record(state, &thread, ThreadStatus::Idle).await?;
    notify(out, state);
    Ok(())
}

/// Points the thread at an earlier session; the next prompt's engine resumes it.
pub async fn resume_session(
    out: &Output,
    state: &AppState,
    input: ResumeThreadSessionInput,
) -> Result<()> {
    let thread_id = input.thread_id;
    let thread = thread_row(state, &thread_id)?;
    if state.store.scratch_for_thread(&thread_id)?.is_some() {
        return Err(Error::Workspace("a scratch has only one session".into()));
    }
    let ended = state.store.thread_session(&thread_id)?;
    state.engines.discard(&thread_id)?;
    match input.session_id.as_deref() {
        Some(session) => state.store.set_thread_session(&thread_id, session)?,
        None => state.store.clear_thread_session(&thread_id)?,
    }
    record_step(
        &state.store,
        &thread_id,
        TimelineStep::SessionResumed {
            conversation: input.conversation_id,
            ended_session: ended,
        },
    )?;
    record(state, &thread, ThreadStatus::Idle).await?;
    notify(out, state);
    Ok(())
}

/// A thread is done without the engine having said so: its findings are fixed, or its review
/// was submitted. The caller tells the webview.
pub async fn mark_idle(state: &AppState, thread: &ThreadRow) -> Result<()> {
    record(state, thread, ThreadStatus::Idle).await
}

/// The status half of a turn ending. An engine's pump calls this as the event goes out, so
/// nobody who hears of the end can read the thread as still running; `ended` follows behind it
/// and has to repeat it, because an engine that exits doesn't go through the pump's save.
///
/// A paused thread stays paused: pausing interrupts the CLI, which answers with a turn end of
/// its own, and idling the thread then would strand its hold (Resume would do nothing and every
/// later prompt would queue behind a pause nobody can see).
pub fn settle(store: &Store, thread_id: &str) -> Result<()> {
    if is_paused(store, thread_id) {
        return Ok(());
    }
    store.set_thread_status(thread_id, ThreadStatus::Idle)
}

fn is_paused(store: &Store, thread_id: &str) -> bool {
    matches!(
        store.thread_by_id(thread_id),
        Ok(Some(thread)) if thread.has_status(ThreadStatus::Paused)
    )
}

/// Everything a turn ending sets off, once per engine event (a turn end, or the engine exiting).
pub async fn ended(out: &Output, state: &AppState, thread_id: &str) {
    if let Err(error) = settle(&state.store, thread_id) {
        log::warn!("settling thread {thread_id}: {error}");
    }
    if let Err(error) = state.store.deny_stale_approvals(thread_id) {
        log::warn!("denying stale approvals on {thread_id}: {error}");
    }
    if let Ok(Some(thread)) = state.store.thread_by_id(thread_id)
        && !thread.has_status(ThreadStatus::Paused)
    {
        sync_manager(state, &thread, ThreadStatus::Idle).await;
    }
    notify(out, state);
    if let Err(error) = crate::review::on_reviewer_turn_end(out, state, thread_id).await {
        log::warn!("reading review findings failed: {error}");
    }
    if let Err(error) = crate::orchestration::on_turn_end(out, state, thread_id).await {
        log::warn!("reporting a thread failed: {error}");
    }
}

/// Runs `ended` for each turn the engines report, one at a time, for the life of the app.
pub async fn listen(out: Output, state: AppState) {
    let mut rx = state.turn_end_notify.subscribe();
    loop {
        match rx.recv().await {
            Ok(thread_id) => ended(&out, &state, &thread_id).await,
            Err(RecvError::Lagged(_)) => continue,
            Err(RecvError::Closed) => break,
        }
    }
}

async fn record(state: &AppState, thread: &ThreadRow, status: ThreadStatus) -> Result<()> {
    state.store.set_thread_status(&thread.id, status)?;
    sync_manager(state, thread, status).await;
    Ok(())
}

/// A scratch thread has no workspace, so it is skipped.
async fn sync_manager(state: &AppState, thread: &ThreadRow, status: ThreadStatus) {
    let is_scratch = state
        .store
        .scratch_for_thread(&thread.id)
        .ok()
        .flatten()
        .is_some();
    if !is_scratch {
        state
            .workspace
            .set_thread(&thread.id, &thread.workspace_id, status)
            .await;
    }
}

fn notify(out: &Output, state: &AppState) {
    state.notify(out, StateChangeKind::WorkspaceStatus);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::EngineCommand;
    use crate::feedback::RecordingOutput;
    use crate::store::types::{ApprovalRow, FindingRow, RepoRecord, WorkspaceRow};
    use crate::workspace::{WorkspaceLifecycle, WorkspaceRecord};
    use tokio::sync::mpsc::UnboundedReceiver;

    type Engine = UnboundedReceiver<EngineCommand>;

    const T: &str = "t1";
    const W: &str = "w1";

    fn thread(id: &str, status: ThreadStatus, parent: Option<&str>) -> ThreadRow {
        ThreadRow {
            id: id.into(),
            workspace_id: W.into(),
            title: "Lead".into(),
            engine: "claude".into(),
            session_id: None,
            status: status.into(),
            used_tokens: None,
            context_size: None,
            cost_usd: None,
            transcript_readonly: false,
            retired: false,
            parent_thread_id: parent.map(str::to_string),
        }
    }

    /// A workspace in both the store and the manager, with one thread in `status` whose engine
    /// is the receiver returned. `workspace` is the workspace's status to start from.
    async fn seeded(status: ThreadStatus, workspace: WorkspaceLifecycle) -> (AppState, Engine) {
        let state = AppState::new(std::env::temp_dir());
        state.store.open_in_memory().unwrap();
        state
            .store
            .upsert_repo(&RepoRecord {
                id: "r1".into(),
                path: "/tmp/app".into(),
                name: "app".into(),
                default_branch: Some("main".into()),
                setup_commands: String::new(),
                run_command: None,
                single_instance: false,
            })
            .unwrap();
        state
            .store
            .upsert_workspace(&WorkspaceRow {
                id: W.into(),
                repo_id: "r1".into(),
                name: "Work".into(),
                branch: "feat/x".into(),
                worktree_path: "/tmp/wt".into(),
                status: workspace.as_str().into(),
                created_at: String::new(),
                summary: None,
                summary_at: None,
                summary_source: "Haiku 4.5".into(),
                kind: None,
                pr_number: None,
                pr_html_url: None,
                modified_files: 0,
                archived_at: None,
            })
            .unwrap();
        state
            .workspace
            .remember(WorkspaceRecord {
                id: W.into(),
                repo_id: "r1".into(),
                repo_path: "/tmp/app".into(),
                name: "Work".into(),
                branch: "feat/x".into(),
                base: "main".into(),
                worktree_path: "/tmp/wt".into(),
                status: workspace,
                version: 1,
                activity: String::new(),
                prov_step: 0,
                setup_failed_command: None,
                setup_failed_exit_code: None,
            })
            .await;
        state.store.upsert_thread(&thread(T, status, None)).unwrap();
        let engine = state.engines.fake_thread(T);
        (state, engine)
    }

    fn thread_status(state: &AppState, id: &str) -> String {
        state.store.thread_by_id(id).unwrap().unwrap().status
    }

    fn stored_workspace(state: &AppState) -> String {
        state.store.workspace_by_id(W).unwrap().unwrap().status
    }

    async fn live_workspace(state: &AppState) -> WorkspaceLifecycle {
        state.workspace.get(W).await.unwrap().status
    }

    fn user_messages(state: &AppState, id: &str) -> usize {
        state
            .store
            .thread_events_after(id, 0, 100)
            .unwrap()
            .iter()
            .filter(|(_, kind, payload)| kind == "message" && payload.contains("\"role\":\"user\""))
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
        recording
            .state_changes()
            .contains(&StateChangeKind::WorkspaceStatus)
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
            let (state, mut engine) = seeded(status, Lifecycle::Idle).await;
            let (recording, out) = RecordingOutput::new();

            let row = thread_row(&state, T).unwrap();
            start(&out, &state, &row, "shown", "prompt").await.unwrap();

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
        let (state, _engine) = seeded(ThreadStatus::Idle, WorkspaceLifecycle::Idle).await;
        let (recording, out) = RecordingOutput::new();
        state.engines.refuse_sends(T);

        let row = thread_row(&state, T).unwrap();
        let sent = start(&out, &state, &row, "shown", "prompt").await;

        assert!(sent.is_err());
        assert_eq!(thread_status(&state, T), "idle");
        assert_eq!(stored_workspace(&state), "idle");
        assert_eq!(live_workspace(&state).await, WorkspaceLifecycle::Idle);
        assert!(!workspace_notified(&recording));
    }

    #[tokio::test]
    async fn a_prompt_queued_while_paused_goes_out_on_resume() {
        let (state, mut engine) = seeded(ThreadStatus::Running, WorkspaceLifecycle::Running).await;
        let (_recording, out) = RecordingOutput::new();

        pause(&out, &state, T).await.unwrap();
        assert!(cancelled(&mut engine));
        assert_eq!(thread_status(&state, T), "paused");
        assert_eq!(live_workspace(&state).await, WorkspaceLifecycle::Idle);

        submit(&out, &state, T, "while paused".into(), Vec::new())
            .await
            .unwrap();
        assert_eq!(thread_status(&state, T), "paused");
        assert!(prompts(&mut engine).is_empty());

        resume(&out, &state, T).await.unwrap();
        assert_eq!(thread_status(&state, T), "running");
        assert_eq!(stored_workspace(&state), "running");
        assert_eq!(live_workspace(&state).await, WorkspaceLifecycle::Running);
        assert_eq!(prompts(&mut engine), ["while paused"]);
    }

    #[tokio::test]
    async fn pause_and_resume_only_apply_to_a_thread_that_can_take_them() {
        let (state, mut engine) = seeded(ThreadStatus::Idle, WorkspaceLifecycle::Idle).await;
        let (recording, out) = RecordingOutput::new();

        pause(&out, &state, T).await.unwrap();
        resume(&out, &state, T).await.unwrap();

        assert_eq!(thread_status(&state, T), "idle");
        assert!(prompts(&mut engine).is_empty());
        assert!(recording.state_changes().is_empty());
    }

    #[tokio::test]
    async fn a_retired_thread_refuses_a_message_and_an_empty_one_is_ignored() {
        let (state, mut engine) = seeded(ThreadStatus::Idle, WorkspaceLifecycle::Idle).await;
        let (recording, out) = RecordingOutput::new();

        submit(&out, &state, T, "   ".into(), Vec::new())
            .await
            .unwrap();
        assert!(recording.state_changes().is_empty());

        let retired = ThreadRow {
            retired: true,
            ..thread(T, ThreadStatus::Idle, None)
        };
        state.store.upsert_thread(&retired).unwrap();
        let refused = submit(&out, &state, T, "hello".into(), Vec::new()).await;

        assert!(refused.is_err());
        assert!(prompts(&mut engine).is_empty());
        assert_eq!(thread_status(&state, T), "idle");
    }

    #[tokio::test]
    async fn stopping_idles_the_thread_at_once_and_cancels_the_engine() {
        let (state, mut engine) = seeded(ThreadStatus::Running, WorkspaceLifecycle::Running).await;
        let (recording, out) = RecordingOutput::new();

        stop(&out, &state, T).await;

        assert!(cancelled(&mut engine));
        assert_eq!(thread_status(&state, T), "idle");
        assert_eq!(stored_workspace(&state), "idle");
        assert_eq!(live_workspace(&state).await, WorkspaceLifecycle::Idle);
        assert!(workspace_notified(&recording));
    }

    #[tokio::test]
    async fn a_new_or_resumed_session_drops_the_engine_and_idles_the_thread() {
        for session in [None, Some("earlier")] {
            let (state, _engine) = seeded(ThreadStatus::Running, WorkspaceLifecycle::Running).await;
            let (_recording, out) = RecordingOutput::new();
            state.store.set_thread_session(T, "current").unwrap();

            match session {
                None => new_session(&out, &state, T).await.unwrap(),
                Some(session) => {
                    let input = ResumeThreadSessionInput {
                        thread_id: T.into(),
                        conversation_id: "c1".into(),
                        session_id: Some(session.into()),
                    };
                    resume_session(&out, &state, input).await.unwrap();
                }
            }

            assert!(!state.engines.has_thread(T));
            assert_eq!(state.store.thread_session(T).unwrap().as_deref(), session);
            assert_eq!(thread_status(&state, T), "idle");
            assert_eq!(stored_workspace(&state), "idle");
            assert_eq!(live_workspace(&state).await, WorkspaceLifecycle::Idle);
        }
    }

    #[tokio::test]
    async fn a_turn_ending_idles_denies_syncs_and_notifies() {
        let (state, _engine) = seeded(ThreadStatus::Running, WorkspaceLifecycle::Idle).await;
        let (recording, out) = RecordingOutput::new();
        state
            .workspace
            .set_thread(T, W, ThreadStatus::Running)
            .await;
        state
            .store
            .upsert_approval(&ApprovalRow {
                id: "a1".into(),
                thread_id: T.into(),
                status: "pending".into(),
                tool: "edit".into(),
                payload: "{}".into(),
            })
            .unwrap();

        ended(&out, &state, T).await;

        assert_eq!(thread_status(&state, T), "idle");
        assert_eq!(stored_workspace(&state), "idle");
        assert_eq!(live_workspace(&state).await, WorkspaceLifecycle::Idle);
        assert!(
            state
                .store
                .pending_approvals_for_thread(T)
                .unwrap()
                .is_empty()
        );
        assert!(workspace_notified(&recording));
    }

    #[tokio::test]
    async fn a_sibling_still_running_keeps_the_workspace_running_after_a_turn_ends() {
        let (state, _engine) = seeded(ThreadStatus::Running, WorkspaceLifecycle::Idle).await;
        let (_recording, out) = RecordingOutput::new();
        state
            .store
            .upsert_thread(&thread("t2", ThreadStatus::Running, None))
            .unwrap();
        state
            .workspace
            .set_thread("t2", W, ThreadStatus::Running)
            .await;

        ended(&out, &state, T).await;

        assert_eq!(thread_status(&state, T), "idle");
        assert_eq!(stored_workspace(&state), "running");
        assert_eq!(live_workspace(&state).await, WorkspaceLifecycle::Running);
    }

    #[tokio::test]
    async fn a_turn_ending_finishes_a_branch_review() {
        let (state, _engine) = seeded(ThreadStatus::Running, WorkspaceLifecycle::Running).await;
        let (recording, out) = RecordingOutput::new();
        state
            .store
            .set_setting(&crate::review::review_status_key(W), "running")
            .unwrap();
        let said = serde_json::json!({
            "type": "messageChunk",
            "role": "agent",
            "text": "Fine.\n\n<cormux-findings>\n[]\n</cormux-findings>",
        });
        state
            .store
            .append_event(T, "message", &said.to_string())
            .unwrap();

        ended(&out, &state, T).await;

        assert!(crate::review::review_is_ready(&state.store, W));
        assert_eq!(recording.toasts().len(), 1);
    }

    /// The report is only delivered to an idle thread, so this also proves the reactions come
    /// after the status: `ended` on the parent is what lets its waiting report through.
    #[tokio::test]
    async fn a_turn_ending_reports_to_the_parent_only_once_the_parent_is_idle() {
        let (state, mut parent) = seeded(ThreadStatus::Running, WorkspaceLifecycle::Running).await;
        let (_recording, out) = RecordingOutput::new();
        state
            .store
            .upsert_thread(&thread("child", ThreadStatus::Running, Some(T)))
            .unwrap();
        let said = serde_json::json!({ "type": "messageChunk", "role": "agent", "text": "done" });
        state
            .store
            .append_event("child", "message", &said.to_string())
            .unwrap();

        ended(&out, &state, "child").await;
        assert!(prompts(&mut parent).is_empty());
        assert_eq!(thread_status(&state, T), "running");

        ended(&out, &state, T).await;
        let reports = prompts(&mut parent);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].contains("done"));
        assert_eq!(thread_status(&state, T), "running");
    }

    async fn with_findings(status: ThreadStatus) -> (AppState, Engine, Vec<String>) {
        let (state, engine) = seeded(status, WorkspaceLifecycle::Idle).await;
        let ids = vec!["f1".to_string(), "f2".to_string()];
        for id in &ids {
            state
                .store
                .upsert_finding(&FindingRow {
                    id: id.clone(),
                    workspace_id: W.into(),
                    severity: "blocking".into(),
                    title: format!("Finding {id}"),
                    file: None,
                    line: None,
                    explanation: "wrong".into(),
                    status: "open".into(),
                    commit_sha: None,
                    sent_to_thread_id: None,
                })
                .unwrap();
        }
        (state, engine, ids)
    }

    fn finding_status(state: &AppState, id: &str) -> String {
        state.store.finding_by_id(id).unwrap().unwrap().status
    }

    #[tokio::test]
    async fn findings_that_cant_be_sent_say_so_and_can_be_sent_again() {
        let (state, _engine, ids) = with_findings(ThreadStatus::Idle).await;
        let (recording, out) = RecordingOutput::new();
        state.engines.refuse_sends(T);

        let sent = crate::findings::send_workspace_findings(&out, &state, W, T, ids).await;

        assert!(sent.is_err(), "the failure reaches the caller");
        assert_eq!(thread_status(&state, T), "idle");
        assert_eq!(stored_workspace(&state), "idle");
        assert_eq!(live_workspace(&state).await, WorkspaceLifecycle::Idle);
        assert_eq!(finding_status(&state, "f1"), "open");
        assert!(recording.toasts().is_empty(), "nothing claims they went");
    }

    #[tokio::test]
    async fn sent_findings_start_a_turn() {
        let (state, mut engine, ids) = with_findings(ThreadStatus::Idle).await;
        let (recording, out) = RecordingOutput::new();

        crate::findings::send_workspace_findings(&out, &state, W, T, ids)
            .await
            .unwrap();

        assert_eq!(prompts(&mut engine).len(), 1);
        assert_eq!(thread_status(&state, T), "running");
        assert_eq!(stored_workspace(&state), "running");
        assert_eq!(live_workspace(&state).await, WorkspaceLifecycle::Running);
        assert_ne!(finding_status(&state, "f1"), "open");
        assert_eq!(recording.toasts().len(), 1);
    }

    #[tokio::test]
    async fn findings_sent_to_a_paused_thread_wait_for_its_resume() {
        let (state, mut engine, ids) = with_findings(ThreadStatus::Paused).await;
        let (_recording, out) = RecordingOutput::new();

        crate::findings::send_workspace_findings(&out, &state, W, T, ids)
            .await
            .unwrap();

        assert_eq!(thread_status(&state, T), "paused");
        assert!(prompts(&mut engine).is_empty());

        resume(&out, &state, T).await.unwrap();
        assert!(prompts(&mut engine)[0].contains("id=f1"));
    }

    #[tokio::test]
    async fn the_last_finding_fixed_idles_the_workspace_too() {
        let (state, _engine, ids) = with_findings(ThreadStatus::Idle).await;
        let (_recording, out) = RecordingOutput::new();
        crate::findings::send_workspace_findings(&out, &state, W, T, ids)
            .await
            .unwrap();
        assert_eq!(stored_workspace(&state), "running");

        for id in ["f1", "f2"] {
            state.store.mark_finding_fixed(id, "abc").unwrap();
            crate::findings::on_finding_marked_fixed(&out, &state, id)
                .await
                .unwrap();
        }

        assert_eq!(thread_status(&state, T), "idle");
        assert_eq!(stored_workspace(&state), "idle");
        assert_eq!(live_workspace(&state).await, WorkspaceLifecycle::Idle);
    }

    #[tokio::test]
    async fn a_branch_review_of_a_paused_thread_waits_for_its_resume() {
        let (state, mut engine) = seeded(ThreadStatus::Paused, WorkspaceLifecycle::Idle).await;
        let (recording, out) = RecordingOutput::new();

        let thread_id = crate::review::start_branch_review(&out, &state, W)
            .await
            .unwrap();

        assert_eq!(thread_id, T);
        assert_eq!(thread_status(&state, T), "paused");
        assert!(prompts(&mut engine).is_empty());
        assert!(workspace_notified(&recording));

        resume(&out, &state, T).await.unwrap();
        assert!(prompts(&mut engine)[0].contains("<cormux-findings>"));
    }

    #[tokio::test]
    async fn a_branch_review_the_engine_refuses_never_starts() {
        let (state, _engine) = seeded(ThreadStatus::Idle, WorkspaceLifecycle::Idle).await;
        let (_recording, out) = RecordingOutput::new();
        state.engines.refuse_sends(T);

        assert!(
            crate::review::start_branch_review(&out, &state, W)
                .await
                .is_err()
        );
        assert_eq!(thread_status(&state, T), "idle");
        assert!(!crate::review::review_is_ready(&state.store, W));
        assert_eq!(
            state
                .store
                .get_setting(&crate::review::review_status_key(W))
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn a_branch_review_is_refused_while_the_thread_is_running() {
        let (state, mut engine) = seeded(ThreadStatus::Running, WorkspaceLifecycle::Running).await;
        let (_recording, out) = RecordingOutput::new();

        assert!(
            crate::review::start_branch_review(&out, &state, W)
                .await
                .is_err()
        );
        assert!(prompts(&mut engine).is_empty());
    }
}
