use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::RwLock;

use super::fake::FakeEngines;
use super::*;
use crate::engines::{ClaudeEngines, ToolRole};
use crate::model::{StopReason, ThreadRole, WorkspaceKind};
use crate::shell_env::ShellEnv;
use crate::store::types::{ApprovalRow, RepoRecord, ThreadRow, WorkspaceRow};
use crate::transcript::{StreamedEvent, TranscriptEvent};

const W: &str = "w1";

struct Setup {
    runtime: ThreadRuntime,
    store: Store,
    transcript: Transcript,
    approvals: Arc<ApprovalBroker>,
    turn_ends: broadcast::Receiver<String>,
}

/// A runtime over `engines`, with one workspace in the temp folder for its threads.
fn setup(engines: Arc<dyn Engines>) -> Setup {
    let store = Store::in_memory().unwrap();
    store
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
    let worktree = std::env::temp_dir();
    store
        .insert_workspace(&WorkspaceRow::new(
            WorkspaceKind::Lead,
            W,
            "r1",
            "Work",
            "feat/x",
            "main",
            worktree.display().to_string(),
        ))
        .unwrap();
    let transcript = Transcript::new(store.clone());
    let approvals = Arc::new(ApprovalBroker::new());
    let (turn_end_notify, turn_ends) = broadcast::channel(8);
    let runtime = ThreadRuntime::new(RuntimeParts {
        engines,
        approvals: approvals.clone(),
        store: store.clone(),
        transcript: transcript.clone(),
        workspaces: Workspaces::new(store.clone()),
        host: CormuxMcp::new(store.clone(), transcript.clone()),
        approval_notify: broadcast::channel(4).0,
        turn_end_notify,
    });
    Setup {
        runtime,
        store,
        transcript,
        approvals,
        turn_ends,
    }
}

fn add_thread(store: &Store, id: &str, status: &str, session: Option<&str>) {
    store
        .upsert_thread(&ThreadRow {
            id: id.into(),
            workspace_id: W.into(),
            title: "Lead".into(),
            role: Some(ThreadRole::Lead),
            engine: "claude".into(),
            session_id: session.map(str::to_string),
            status: status.into(),
            used_tokens: None,
            context_size: None,
            cost_usd: None,
            transcript_readonly: false,
            retired: false,
            parent_thread_id: None,
        })
        .unwrap();
}

/// Claude engines whose `claude` on PATH is the mock CLI.
fn mock_claude() -> (Arc<dyn Engines>, tempfile::TempDir) {
    use std::os::unix::fs::PermissionsExt;
    let script =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/engines/claude/mock_claude.py");
    let dir = tempfile::tempdir().unwrap();
    let shim = dir.path().join("claude");
    std::fs::write(
        &shim,
        format!("#!/bin/sh\nexec python3 '{}' \"$@\"\n", script.display()),
    )
    .unwrap();
    std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        dir.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let env = ShellEnv::from_vars([("PATH".to_string(), path)]);
    (
        Arc::new(ClaudeEngines::new(Arc::new(RwLock::new(env)))),
        dir,
    )
}

async fn next_event(events: &mut broadcast::Receiver<StreamedEvent>) -> AgentEvent {
    loop {
        let streamed = tokio::time::timeout(Duration::from_secs(10), events.recv())
            .await
            .expect("an event within 10s")
            .unwrap();
        if let TranscriptEvent::Agent(event) = streamed.event {
            return event;
        }
    }
}

#[tokio::test]
async fn stop_returns_as_soon_as_the_engine_is_down() {
    let fake = FakeEngines::default();
    let setup = setup(Arc::new(fake.clone()));
    add_thread(&setup.store, "t1", "idle", None);
    let mut engine = fake.engine("t1");
    setup.runtime.ensure("t1", W).await.unwrap();
    tokio::spawn(async move {
        while let Some(command) = engine.recv().await {
            if matches!(command, EngineCommand::Shutdown) {
                break;
            }
        }
    });
    let started = Instant::now();
    setup
        .runtime
        .stop_threads(&["t1".to_string()])
        .await
        .unwrap();
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(!setup.runtime.has_thread("t1"));
}

#[tokio::test]
async fn only_a_quiet_resumable_idle_engine_is_reaped() {
    let fake = FakeEngines::default();
    let setup = setup(Arc::new(fake.clone()));
    let mut engines = Vec::new();
    for (id, status, session) in [
        ("idle", "idle", Some("s")),
        ("running", "running", Some("s")),
        ("held", "idle", Some("s")),
        ("asking", "idle", Some("s")),
        ("fresh", "idle", None),
    ] {
        add_thread(&setup.store, id, status, session);
        engines.push(fake.engine(id));
        setup.runtime.ensure(id, W).await.unwrap();
    }
    setup.runtime.hold_thread("held").unwrap();
    setup
        .store
        .upsert_approval(&ApprovalRow {
            id: "a1".into(),
            thread_id: "asking".into(),
            status: "pending".into(),
            tool: "edit".into(),
            payload: "{}".into(),
        })
        .unwrap();

    assert!(setup.runtime.reap_idle(Duration::from_secs(60)).is_empty());
    assert_eq!(
        setup.runtime.reap_idle(Duration::ZERO),
        vec!["idle".to_string()]
    );
    assert!(!setup.runtime.has_thread("idle"));
    assert!(setup.runtime.has_thread("running"));
}

#[tokio::test]
async fn a_session_starts_with_the_threads_choices_and_the_cormux_tools() {
    let fake = FakeEngines::default();
    let setup = setup(Arc::new(fake.clone()));
    add_thread(&setup.store, "t1", "idle", Some("sess-9"));
    setup.store.set_thread_model("t1", Some("opus")).unwrap();
    setup.runtime.ensure("t1", W).await.unwrap();

    let launched = fake.launches().remove(0);
    assert_eq!(launched.resume.as_deref(), Some("sess-9"));
    assert_eq!(launched.model.as_deref(), Some("opus"));
    assert!(launched.instructions.contains("git reset --hard <branch>"));
    assert!(launched.instructions.contains(&crate::media::instruction()));
    let tools = launched.host_tools.unwrap();
    assert_eq!(tools.server_name, crate::mcp::SERVER_NAME);
    assert_eq!(tools.disallowed, ["attach_entity"], "References is off");
    assert!(!tools.instructions.contains("attach_entity"));
}

#[tokio::test]
async fn a_turn_end_idles_the_thread_drops_its_approvals_and_is_published_once() {
    let fake = FakeEngines::default();
    let mut setup = setup(Arc::new(fake.clone()));
    add_thread(&setup.store, "t1", "running", None);
    setup.runtime.ensure("t1", W).await.unwrap();
    let mut events = setup.transcript.subscribe("t1");

    let (respond, answered) = tokio::sync::oneshot::channel();
    fake.report(
        "t1",
        EngineOutput::Permission(PermissionRequest {
            id: "p1".into(),
            tool_call_id: Some("toolu_1".into()),
            tool: "Bash".into(),
            role: ToolRole::Command {
                command: "ls".into(),
                description: None,
            },
            detail: Some("ls".into()),
            questions: Vec::new(),
            respond,
        }),
    );
    let saved = loop {
        let streamed = events.recv().await.unwrap();
        if let TranscriptEvent::Agent(AgentEvent::Permission { .. }) = streamed.event {
            break streamed.seq;
        }
    };
    assert!(saved.is_some(), "a pending approval is in the transcript");
    assert_eq!(
        setup
            .store
            .pending_approvals_for_thread("t1")
            .unwrap()
            .len(),
        1
    );

    fake.report(
        "t1",
        EngineOutput::Event(AgentEvent::TurnEnd {
            stop_reason: StopReason::EndTurn,
            error: None,
        }),
    );
    assert_eq!(setup.turn_ends.recv().await.unwrap(), "t1");
    assert!(answered.await.is_err(), "the engine stopped waiting");
    let thread = setup.store.thread_by_id("t1").unwrap().unwrap();
    assert!(thread.has_status(ThreadStatus::Idle));
    assert!(
        setup
            .store
            .pending_approvals_for_thread("t1")
            .unwrap()
            .is_empty()
    );
    assert!(setup.turn_ends.try_recv().is_err());
}

#[tokio::test]
async fn each_turns_cost_adds_to_the_threads_total() {
    let fake = FakeEngines::default();
    let setup = setup(Arc::new(fake.clone()));
    add_thread(&setup.store, "t1", "running", None);
    setup.runtime.ensure("t1", W).await.unwrap();
    let mut events = setup.transcript.subscribe("t1");
    for cost in [0.25, 0.5] {
        fake.report(
            "t1",
            EngineOutput::Usage {
                used_tokens: 10,
                context_size: 100,
                turn_cost_usd: Some(cost),
            },
        );
        next_event(&mut events).await;
    }
    let thread = setup.store.thread_by_id("t1").unwrap().unwrap();
    assert_eq!(thread.cost_usd, Some(0.75));
}

#[tokio::test]
async fn questions_wait_for_answers_even_with_auto_approve_on() {
    let (engines, _shim) = mock_claude();
    let setup = setup(engines);
    add_thread(&setup.store, "t-ask", "idle", None);
    setup.store.set_setting("runEverything", "true").unwrap();
    let mut events = setup.transcript.subscribe("t-ask");
    setup.runtime.ensure("t-ask", W).await.unwrap();
    setup.runtime.prompt("t-ask", "ask me something").unwrap();
    let id = loop {
        if let AgentEvent::Permission {
            id, auto_approved, ..
        } = next_event(&mut events).await
        {
            assert!(!auto_approved);
            break id;
        }
    };

    let row = setup.store.get_approval(&id).unwrap().unwrap();
    assert_eq!(row.tool, crate::approvals::QUESTION_TOOL);
    assert!(row.payload.contains("Which colour?"));

    let answers = std::collections::BTreeMap::from([(
        "Which colour?".to_string(),
        "Green, my own".to_string(),
    )]);
    setup
        .approvals
        .complete(&id, ApprovalDecision::Answered { answers })
        .unwrap();

    // The mock echoes the decision back as a message.
    let mut reply = String::new();
    loop {
        match next_event(&mut events).await {
            AgentEvent::MessageChunk { text, .. } => reply.push_str(&text),
            AgentEvent::TurnEnd { .. } => break,
            AgentEvent::EngineExited { .. } => panic!("exited early"),
            _ => {}
        }
    }
    setup.runtime.stop("t-ask").await.unwrap();
    let echoed: serde_json::Value = serde_json::from_str(&reply).unwrap();
    assert_eq!(echoed["behavior"], "allow");
    assert_eq!(
        echoed["updatedInput"]["answers"]["Which colour?"],
        "Green, my own"
    );
}

#[tokio::test]
async fn a_turn_ends_only_after_its_background_agents_do() {
    let (engines, _shim) = mock_claude();
    let mut setup = setup(engines);
    add_thread(&setup.store, "t-bg", "idle", None);
    setup.runtime.ensure("t-bg", W).await.unwrap();
    setup.runtime.prompt("t-bg", "background agent").unwrap();
    // The mock's first turn finishes at once; its agent takes a moment longer.
    let early = tokio::time::timeout(Duration::from_millis(300), setup.turn_ends.recv()).await;
    assert!(early.is_err(), "turn ended with an agent still running");
    let thread_id = tokio::time::timeout(Duration::from_secs(10), setup.turn_ends.recv())
        .await
        .expect("turn end within 10s")
        .unwrap();
    assert_eq!(thread_id, "t-bg");
}

#[tokio::test]
async fn claude_mock_prompt_permission_and_cancel() {
    let (engines, _shim) = mock_claude();
    let setup = setup(engines);
    add_thread(&setup.store, "t1", "idle", None);
    let mut events = setup.transcript.subscribe("t1");
    setup.runtime.ensure("t1", W).await.unwrap();
    setup
        .runtime
        .prompt("t1", "first: run echo spike-ok")
        .unwrap();

    let permission_id = loop {
        if let AgentEvent::Permission { id, role, tool, .. } = next_event(&mut events).await {
            assert_eq!(tool, "Bash");
            assert!(
                matches!(role, ToolRole::Command { command, .. } if command == "echo spike-ok")
            );
            break id;
        }
    };
    setup
        .approvals
        .resolve(&permission_id, true, None)
        .await
        .unwrap();
    loop {
        match next_event(&mut events).await {
            AgentEvent::TurnEnd { stop_reason, .. } => {
                assert_eq!(stop_reason, StopReason::EndTurn);
                break;
            }
            AgentEvent::EngineExited { .. } => panic!("exited early"),
            _ => {}
        }
    }
    assert_eq!(
        setup.store.thread_session("t1").unwrap().as_deref(),
        Some("sess-1")
    );

    setup.runtime.prompt("t1", "second: count slowly").unwrap();
    setup.runtime.cancel("t1").unwrap();
    loop {
        if let AgentEvent::TurnEnd { stop_reason, .. } = next_event(&mut events).await {
            assert_ne!(stop_reason, StopReason::EndTurn);
            break;
        }
    }
    setup.runtime.stop("t1").await.unwrap();
}

/// Starts a real Haiku session and has it launch one background agent, returning the
/// session's events once they begin.
async fn live_background_agent(thread_id: &str) -> (Setup, broadcast::Receiver<StreamedEvent>) {
    let engines = Arc::new(ClaudeEngines::new(Arc::new(RwLock::new(ShellEnv::new()))));
    let setup = setup(engines);
    add_thread(&setup.store, thread_id, "idle", None);
    setup
        .store
        .set_thread_model(thread_id, Some("haiku"))
        .unwrap();
    setup.store.set_setting("runEverything", "true").unwrap();
    let events = setup.transcript.subscribe(thread_id);
    setup.runtime.ensure(thread_id, W).await.unwrap();
    setup
        .runtime
        .prompt(
            thread_id,
            "Use the Agent tool with run_in_background set to true and subagent_type \
             general-purpose. Its prompt: run the Bash command \
             `python3 -c \"import time; time.sleep(15)\"` in the foreground, then reply ok. \
             Once it is launched, end your turn with the single word launched.",
        )
        .unwrap();
    (setup, events)
}

async fn next_live_event(events: &mut broadcast::Receiver<StreamedEvent>) -> AgentEvent {
    loop {
        let streamed = tokio::time::timeout(Duration::from_secs(120), events.recv())
            .await
            .expect("an event within 120s")
            .unwrap();
        if let TranscriptEvent::Agent(event) = streamed.event {
            return event;
        }
    }
}

/// Needs a signed-in `claude` on PATH, and spends a few small Haiku turns.
#[tokio::test]
#[ignore]
async fn live_claude_holds_the_turn_for_a_background_agent() {
    let (setup, mut events) = live_background_agent("live-bg").await;
    let mut agent = None;
    let mut agent_finished = false;
    loop {
        match next_live_event(&mut events).await {
            AgentEvent::ToolCall {
                id,
                role: ToolRole::Subagent { .. },
                ..
            } => {
                agent = Some(id);
            }
            AgentEvent::ToolCallUpdate { id, status, .. }
                if Some(&id) == agent.as_ref() && status.is_some() =>
            {
                agent_finished = true;
            }
            AgentEvent::TurnEnd { .. } => break,
            _ => {}
        }
    }
    setup.runtime.stop("live-bg").await.unwrap();
    assert!(agent.is_some(), "never launched an agent");
    assert!(agent_finished, "turn ended before its agent did");
}

/// Needs a signed-in `claude` on PATH, and spends a few small Haiku turns.
#[tokio::test]
#[ignore]
async fn live_claude_stop_reaches_a_background_agent() {
    let (setup, mut events) = live_background_agent("live-stop").await;
    let mut agent = None;
    // The agent's first step means it is under way and the main turn is wrapping up.
    loop {
        match next_live_event(&mut events).await {
            AgentEvent::ToolCall {
                id,
                role: ToolRole::Subagent { .. },
                ..
            } => {
                agent = Some(id);
            }
            AgentEvent::ToolCall { parent_id, .. } if parent_id.is_some() => break,
            AgentEvent::TurnEnd { .. } => panic!("turn ended before its agent did"),
            _ => {}
        }
    }
    tokio::time::sleep(Duration::from_secs(3)).await;
    setup.runtime.cancel("live-stop").unwrap();
    // A foreground sleep this long would still be going if nothing reached the agent.
    let stopped = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            if let AgentEvent::ToolCallUpdate { id, status, .. } =
                next_live_event(&mut events).await
                && Some(&id) == agent.as_ref()
                && status.is_some()
            {
                break;
            }
        }
    })
    .await;
    setup.runtime.stop("live-stop").await.unwrap();
    assert!(stopped.is_ok(), "the agent kept running after Stop");
}

/// Needs a signed-in `claude` on PATH, and spends one small Haiku turn.
#[tokio::test]
#[ignore]
async fn live_claude_reaches_the_cormux_tools() {
    let engines = Arc::new(ClaudeEngines::new(Arc::new(RwLock::new(ShellEnv::new()))));
    let setup = setup(engines);
    add_thread(&setup.store, "live", "idle", None);
    setup.store.set_thread_model("live", Some("haiku")).unwrap();
    let mut events = setup.transcript.subscribe("live");
    setup.runtime.ensure("live", W).await.unwrap();
    setup
        .runtime
        .prompt(
            "live",
            "Call mcp__cormux__get_thread with id \"nonexistent\", then quote its error.",
        )
        .unwrap();
    let mut reply = String::new();
    let mut used_tool = false;
    loop {
        match next_live_event(&mut events).await {
            AgentEvent::ToolCall {
                role: ToolRole::Mcp { action, .. },
                ..
            } => {
                used_tool |= action == "Get thread";
            }
            AgentEvent::Permission { .. } => panic!("a read tool asked for approval"),
            AgentEvent::MessageChunk { text, .. } => reply.push_str(&text),
            AgentEvent::TurnEnd { .. } => break,
            _ => {}
        }
    }
    setup.runtime.stop("live").await.unwrap();
    assert!(used_tool, "never called the tool: {reply}");
    assert!(reply.contains("unknown thread"), "{reply}");
}
