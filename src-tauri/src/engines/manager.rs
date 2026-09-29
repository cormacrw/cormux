use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::{RwLock, broadcast, mpsc};

use super::acp;
use super::claude::map as claude_map;
use super::claude::{CanUseTool, ClaudeSession, Event, PermissionDecision, SpawnOptions};
use super::command::EngineCommand;
use super::detect::{self, EngineStatus};
use super::events::{AgentEvent, EngineKind, ToolCallStatus, ToolKind};
use crate::approvals::{ApprovalBroker, ApprovalDecision};
use crate::error::{Error, Result};
use crate::shell_env::ShellEnv;
use crate::store::Store;

const EVENT_CAP: usize = 256;
const STOP_GRACE: Duration = Duration::from_secs(5);

#[derive(Default)]
struct HoldState {
    paused: bool,
    queued: Vec<String>,
}

#[derive(Clone)]
pub struct EngineRegistry {
    env: Arc<RwLock<ShellEnv>>,
    approvals: Arc<ApprovalBroker>,
    pub(crate) store: Store,
    approval_notify: tokio::sync::broadcast::Sender<()>,
    threads: Arc<Mutex<HashMap<String, ThreadSlot>>>,
    holds: Arc<Mutex<HashMap<String, HoldState>>>,
}

struct ThreadSlot {
    #[allow(dead_code)]
    kind: EngineKind,
    commands: mpsc::UnboundedSender<EngineCommand>,
    events: broadcast::Sender<AgentEvent>,
    session_id: Arc<Mutex<Option<String>>>,
    pending_seed: Arc<Mutex<Option<String>>>,
}

#[derive(Debug, Clone)]
pub struct SpawnSpec {
    pub thread_id: String,
    pub kind: EngineKind,
    pub cwd: PathBuf,
    pub resume: Option<String>,
    pub override_argv: Option<Vec<String>>,
    pub auto_approve_readonly: bool,
    pub auto_approve_all: bool,
    /// Scratches: the engine's own read-only mode, and edits are denied without asking.
    pub read_only: bool,
}

/// Sent back to an agent that tries to edit inside a read-only scratch.
pub const READ_ONLY_DENIAL: &str =
    "This is a read-only scratch. Do not edit files; answer in the conversation instead.";

impl EngineRegistry {
    pub fn new(
        env: Arc<RwLock<ShellEnv>>,
        approvals: Arc<ApprovalBroker>,
        store: Store,
        approval_notify: tokio::sync::broadcast::Sender<()>,
    ) -> Self {
        Self {
            env,
            approvals,
            store,
            approval_notify,
            threads: Arc::new(Mutex::new(HashMap::new())),
            holds: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn detect(&self) -> Result<Vec<EngineStatus>> {
        let env = self.env.read().await;
        detect::detect_engines(&env).await
    }

    pub fn subscribe(&self, thread_id: &str) -> Result<broadcast::Receiver<AgentEvent>> {
        let threads = self
            .threads
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?;
        let slot = threads
            .get(thread_id)
            .ok_or_else(|| Error::Engine(format!("no engine for thread {thread_id}")))?;
        Ok(slot.events.subscribe())
    }

    pub fn has_thread(&self, thread_id: &str) -> bool {
        self.threads
            .lock()
            .ok()
            .is_some_and(|threads| threads.contains_key(thread_id))
    }

    pub fn session_id(&self, thread_id: &str) -> Result<Option<String>> {
        let threads = self
            .threads
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?;
        Ok(threads
            .get(thread_id)
            .and_then(|slot| slot.session_id.lock().ok()?.clone()))
    }

    pub async fn spawn(&self, spec: SpawnSpec) -> Result<()> {
        let _ = self.stop(&spec.thread_id).await;
        let (events, _) = broadcast::channel(EVENT_CAP);
        let resume = spec
            .resume
            .clone()
            .or_else(|| self.store.thread_session(&spec.thread_id).ok().flatten());
        let mut spec = spec;
        spec.resume = resume;
        let session_id = Arc::new(Mutex::new(spec.resume.clone()));
        let pending_seed = Arc::new(Mutex::new(None));
        let env = self.env.read().await.clone();
        let commands = match spec.kind {
            EngineKind::Claude => {
                self.spawn_claude(
                    &spec,
                    env,
                    events.clone(),
                    session_id.clone(),
                    pending_seed.clone(),
                )
                .await?
            }
            EngineKind::Cursor | EngineKind::Codex | EngineKind::Gemini => self.spawn_acp(
                &spec,
                env,
                events.clone(),
                session_id.clone(),
                pending_seed.clone(),
            ),
        };

        self.threads
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?
            .insert(
                spec.thread_id.clone(),
                ThreadSlot {
                    kind: spec.kind,
                    commands,
                    events,
                    session_id,
                    pending_seed,
                },
            );
        Ok(())
    }

    pub fn prompt(&self, thread_id: &str, text: impl Into<String>) -> Result<()> {
        let mut text = text.into();
        if let Some(seed) = self.take_seed(thread_id)? {
            text = format!(
                "Previous session could not be resumed. Summary of the earlier transcript:\n{seed}\n\n---\n{text}"
            );
        }
        self.send(thread_id, EngineCommand::Prompt(text))
    }

    fn take_seed(&self, thread_id: &str) -> Result<Option<String>> {
        let threads = self
            .threads
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?;
        let Some(slot) = threads.get(thread_id) else {
            return Ok(None);
        };
        Ok(slot
            .pending_seed
            .lock()
            .ok()
            .and_then(|mut seed| seed.take()))
    }

    pub fn cancel(&self, thread_id: &str) -> Result<()> {
        self.send(thread_id, EngineCommand::Cancel)
    }

    pub fn thread_is_held(&self, thread_id: &str) -> bool {
        self.holds
            .lock()
            .ok()
            .and_then(|holds| holds.get(thread_id).map(|row| row.paused))
            .unwrap_or(false)
    }

    pub fn hold_thread(&self, thread_id: &str) -> Result<()> {
        let _ = self.cancel(thread_id);
        let mut holds = self
            .holds
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?;
        holds
            .entry(thread_id.to_string())
            .or_default()
            .paused = true;
        Ok(())
    }

    pub fn release_thread(&self, thread_id: &str) -> Result<Option<String>> {
        let mut holds = self
            .holds
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?;
        let slot = holds.entry(thread_id.to_string()).or_default();
        slot.paused = false;
        let queued = std::mem::take(&mut slot.queued);
        if queued.is_empty() {
            return Ok(Some("continue".into()));
        }
        Ok(Some(queued.join("\n\n")))
    }

    pub fn enqueue_while_held(&self, thread_id: &str, text: String) -> Result<()> {
        let mut holds = self
            .holds
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?;
        holds
            .entry(thread_id.to_string())
            .or_default()
            .queued
            .push(text);
        Ok(())
    }

    pub fn submit_prompt(&self, thread_id: &str, text: String, held: bool) -> Result<()> {
        if held || self.thread_is_held(thread_id) {
            return self.enqueue_while_held(thread_id, text);
        }
        self.prompt(thread_id, text)
    }

    pub async fn stop(&self, thread_id: &str) -> Result<()> {
        let commands = {
            let mut threads = self
                .threads
                .lock()
                .map_err(|error| Error::Engine(error.to_string()))?;
            threads.remove(thread_id).map(|slot| slot.commands)
        };
        if let Some(commands) = commands {
            let _ = commands.send(EngineCommand::Shutdown);
            tokio::time::sleep(STOP_GRACE).await;
        }
        Ok(())
    }

    pub async fn stop_all(&self) -> Result<()> {
        let ids: Vec<String> = self
            .threads
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?
            .keys()
            .cloned()
            .collect();
        for id in ids {
            let _ = self.stop(&id).await;
        }
        Ok(())
    }

    pub async fn stop_threads(&self, thread_ids: &[String]) -> Result<()> {
        for id in thread_ids {
            let _ = self.stop(id).await;
        }
        Ok(())
    }

    fn send(&self, thread_id: &str, command: EngineCommand) -> Result<()> {
        let threads = self
            .threads
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?;
        let slot = threads
            .get(thread_id)
            .ok_or_else(|| Error::Engine(format!("no engine for thread {thread_id}")))?;
        slot.commands
            .send(command)
            .map_err(|_| Error::Engine("engine process is gone".into()))
    }

    async fn spawn_claude(
        &self,
        spec: &SpawnSpec,
        env: ShellEnv,
        events: broadcast::Sender<AgentEvent>,
        session_id: Arc<Mutex<Option<String>>>,
        pending_seed: Arc<Mutex<Option<String>>>,
    ) -> Result<mpsc::UnboundedSender<EngineCommand>> {
        let _ = pending_seed;
        let (binary, extra_args, use_default_args) = if let Some(argv) = &spec.override_argv {
            let mut argv = argv.clone();
            let binary = PathBuf::from(argv.remove(0));
            (binary, argv, false)
        } else {
            let binary = detect::resolve_binary(&env, spec.kind.binaries())
                .unwrap_or_else(|| PathBuf::from("claude"));
            // Plan mode reads without asking and never edits.
            let extra = if spec.read_only {
                vec!["--permission-mode".into(), "plan".into()]
            } else {
                Vec::new()
            };
            (binary, extra, true)
        };

        let mut options = SpawnOptions {
            binary,
            cwd: Some(spec.cwd.clone()),
            resume: spec.resume.clone(),
            extra_args,
            use_default_args,
            env: Some(env.vars().clone()),
        };
        if options.env.as_ref().is_some_and(|vars| vars.is_empty()) {
            options.env = None;
        }

        let session = ClaudeSession::spawn(options).await?;
        let (tx, rx) = mpsc::unbounded_channel();
        let approvals = self.approvals.clone();
        let store = self.store.clone();
        let thread_id = spec.thread_id.clone();
        let auto_ro = spec.auto_approve_readonly;
        let auto_all = spec.auto_approve_all;
        let read_only = spec.read_only;
        let approval_notify = self.approval_notify.clone();
        tokio::spawn(async move {
            run_claude(
                session,
                rx,
                events,
                approvals,
                store,
                thread_id,
                session_id,
                ClaudePolicy {
                    auto_ro,
                    auto_all,
                    read_only,
                },
                approval_notify,
            )
            .await;
        });
        Ok(tx)
    }

    fn spawn_acp(
        &self,
        spec: &SpawnSpec,
        env: ShellEnv,
        events: broadcast::Sender<AgentEvent>,
        session_id: Arc<Mutex<Option<String>>>,
        pending_seed: Arc<Mutex<Option<String>>>,
    ) -> mpsc::UnboundedSender<EngineCommand> {
        let argv = spec
            .override_argv
            .clone()
            .unwrap_or_else(|| resolved_acp_argv(spec.kind, &env));
        acp::start(acp::AcpSpawn {
            argv,
            cwd: spec.cwd.clone(),
            resume: spec.resume.clone(),
            env,
            events,
            approvals: self.approvals.clone(),
            auto_approve_readonly: spec.auto_approve_readonly,
            auto_approve_all: spec.auto_approve_all,
            read_only: spec.read_only,
            session_id,
            store: self.store.clone(),
            thread_id: spec.thread_id.clone(),
            pending_seed,
            approval_notify: self.approval_notify.clone(),
        })
    }
}

pub fn default_acp_argv(kind: EngineKind) -> Vec<String> {
    match kind {
        EngineKind::Cursor => vec!["agent".into(), "acp".into()],
        EngineKind::Gemini => vec!["gemini".into(), "--acp".into()],
        EngineKind::Codex => vec!["codex-acp".into()],
        EngineKind::Claude => vec!["claude".into()],
    }
}

fn resolved_acp_argv(kind: EngineKind, env: &ShellEnv) -> Vec<String> {
    let mut argv = default_acp_argv(kind);
    if let Some(binary) = detect::resolve_binary(env, kind.binaries()) {
        argv[0] = binary.display().to_string();
    }
    argv
}

async fn run_claude(
    mut session: ClaudeSession,
    mut commands: mpsc::UnboundedReceiver<EngineCommand>,
    events: broadcast::Sender<AgentEvent>,
    approvals: Arc<ApprovalBroker>,
    store: Store,
    thread_id: String,
    session_id: Arc<Mutex<Option<String>>>,
    policy: ClaudePolicy,
    approval_notify: tokio::sync::broadcast::Sender<()>,
) {
    let (answer_tx, mut answer_rx) =
        mpsc::unbounded_channel::<(String, ApprovalDecision, serde_json::Value)>();
    loop {
        tokio::select! {
            command = commands.recv() => {
                match command {
                    Some(EngineCommand::Prompt(text)) => {
                        if let Err(error) = session.send_user(text).await {
                            log::warn!("claude prompt: {error}");
                        }
                    }
                    Some(EngineCommand::Cancel) => {
                        let _ = session.interrupt().await;
                    }
                    Some(EngineCommand::Shutdown) | None => {
                        let _ = session.shutdown().await;
                        break;
                    }
                }
            }
            event = session.next_event() => {
                match event {
                    Ok(Some(raw)) => {
                        if let Event::Init { session_id: id } = &raw {
                            *session_id.lock().unwrap() = Some(id.clone());
                            let _ = store.set_thread_session(&thread_id, id);
                        }
                        if let Some(usage) = claude_map::usage_from_result(&raw) {
                            persist_event(&store, &thread_id, &usage);
                            let _ = events.send(usage.clone());
                            if let AgentEvent::Usage {
                                used_tokens,
                                context_size,
                                cost_usd,
                            } = usage
                            {
                                let _ = store.set_thread_usage(
                                    &thread_id,
                                    used_tokens,
                                    context_size,
                                    cost_usd,
                                );
                            }
                        }
                        if let Event::CanUseTool(req) = &raw {
                            handle_claude_permission(
                                &mut session,
                                req,
                                &events,
                                &approvals,
                                &store,
                                &thread_id,
                                &approval_notify,
                                policy,
                                answer_tx.clone(),
                            )
                            .await;
                        } else if let Some(mapped) = claude_map::map_claude_event(&raw) {
                            persist_event(&store, &thread_id, &mapped);
                            let _ = events.send(mapped);
                        }
                    }
                    Ok(None) => {
                        let _ = events.send(AgentEvent::EngineExited { code: None });
                        break;
                    }
                    Err(error) => {
                        let _ = events.send(AgentEvent::TurnEnd {
                            stop_reason: "error".into(),
                            error: Some(error.to_string()),
                        });
                        break;
                    }
                }
            }
            Some((id, decision, input)) = answer_rx.recv() => {
                let perm = match decision {
                    ApprovalDecision::Approved => PermissionDecision::Allow {
                        updated_input: input,
                    },
                    ApprovalDecision::Denied { message } => PermissionDecision::Deny { message },
                };
                let _ = session.answer_tool(&id, perm).await;
            }
        }
    }
}

#[derive(Clone, Copy)]
struct ClaudePolicy {
    auto_ro: bool,
    auto_all: bool,
    read_only: bool,
}

async fn handle_claude_permission(
    session: &mut ClaudeSession,
    req: &CanUseTool,
    events: &broadcast::Sender<AgentEvent>,
    approvals: &Arc<ApprovalBroker>,
    store: &Store,
    thread_id: &str,
    approval_notify: &tokio::sync::broadcast::Sender<()>,
    policy: ClaudePolicy,
    answer_tx: mpsc::UnboundedSender<(String, ApprovalDecision, serde_json::Value)>,
) {
    let kind = ToolKind::from_claude_tool(&req.tool_name);
    // Leaving plan mode is how Claude asks to start editing, so a scratch always says no.
    if policy.read_only && (kind.is_edit() || req.tool_name == "ExitPlanMode") {
        let _ = session
            .answer_tool(
                &req.request_id,
                PermissionDecision::Deny {
                    message: READ_ONLY_DENIAL.into(),
                },
            )
            .await;
        return;
    }
    let mut mapped =
        claude_map::map_claude_event(&Event::CanUseTool(req.clone())).expect("permission maps");
    if policy.auto_all || (policy.auto_ro && kind.is_readonly()) {
        if let AgentEvent::Permission { auto_approved, .. } = &mut mapped {
            *auto_approved = true;
        }
        let _ = events.send(mapped);
        let _ = events.send(claude_map::tool_event_from_permission(
            req,
            ToolCallStatus::Completed,
        ));
        let _ = session
            .answer_tool(
                &req.request_id,
                PermissionDecision::Allow {
                    updated_input: req.input.clone(),
                },
            )
            .await;
        return;
    }

    let _ = events.send(mapped.clone());
    let _ = events.send(claude_map::tool_event_from_permission(
        req,
        ToolCallStatus::Pending,
    ));
    if let Err(error) =
        crate::approvals::record_pending_permission(store, thread_id, &mapped, None)
    {
        log::warn!("record approval: {error}");
    } else {
        let _ = approval_notify.send(());
    }
    let _ = events.send(AgentEvent::CurrentTool {
        id: req.tool_use_id.clone(),
        title: req.tool_name.clone(),
    });
    let id = req.request_id.clone();
    let input = req.input.clone();
    let rx = approvals.register(id.clone());
    tokio::spawn(async move {
        let decision = rx.await.unwrap_or(ApprovalDecision::Denied {
            message: "approval dropped".into(),
        });
        let _ = answer_tx.send((id, decision, input));
    });
}

fn persist_event(store: &Store, thread_id: &str, event: &AgentEvent) {
    let kind = match event {
        AgentEvent::SessionStarted { .. } => "session",
        AgentEvent::MessageChunk { .. } => "message",
        AgentEvent::ToolCall { .. } => "tool",
        AgentEvent::Plan { .. } => "plan",
        AgentEvent::Permission { .. } => "permission",
        AgentEvent::CurrentTool { .. } => "current_tool",
        AgentEvent::Usage { .. } => "usage",
        AgentEvent::TurnEnd { .. } => "turn_end",
        AgentEvent::EngineExited { .. } => "exit",
    };
    if let Ok(payload) = serde_json::to_string(event) {
        let _ = store.append_event(thread_id, kind, &payload);
    }
    if matches!(event, AgentEvent::TurnEnd { .. } | AgentEvent::EngineExited { .. }) {
        let _ = store.mark_thread_idle(thread_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approvals::ApprovalBroker;
    use crate::shell_env::ShellEnv;
    use crate::store::Store;
    use std::path::PathBuf;

    fn registry() -> EngineRegistry {
        let env = Arc::new(RwLock::new(ShellEnv::new()));
        let approvals = Arc::new(ApprovalBroker::new());
        let store = Store::new();
        store.open_in_memory().unwrap();
        let (approval_notify, _) = tokio::sync::broadcast::channel(4);
        EngineRegistry::new(env, approvals, store, approval_notify)
    }

    fn mock_claude_argv() -> Vec<String> {
        let script =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/engines/claude/mock_claude.py");
        vec!["python3".into(), script.display().to_string()]
    }

    #[tokio::test]
    async fn claude_mock_prompt_permission_and_cancel() {
        let engines = registry();
        let approvals = engines.approvals.clone();
        engines
            .spawn(SpawnSpec {
                thread_id: "t1".into(),
                kind: EngineKind::Claude,
                cwd: std::env::temp_dir(),
                resume: None,
                override_argv: Some(mock_claude_argv()),
                auto_approve_readonly: true,
                auto_approve_all: false,
                read_only: false,
            })
            .await
            .unwrap();

        let mut events = engines.subscribe("t1").unwrap();
        engines.prompt("t1", "first: run echo spike-ok").unwrap();

        let permission_id = loop {
            let event = events.recv().await.unwrap();
            if let AgentEvent::Permission { id, kind, .. } = event {
                assert_eq!(kind, ToolKind::Execute);
                break id;
            }
        };
        approvals
            .resolve(&permission_id, true, None)
            .await
            .unwrap();

        loop {
            match events.recv().await.unwrap() {
                AgentEvent::TurnEnd { stop_reason, .. } => {
                    assert_eq!(stop_reason, "end_turn");
                    break;
                }
                AgentEvent::EngineExited { .. } => panic!("exited early"),
                _ => {}
            }
        }

        engines.prompt("t1", "second: count slowly").unwrap();
        engines.cancel("t1").unwrap();
        loop {
            match events.recv().await.unwrap() {
                AgentEvent::TurnEnd { stop_reason, .. } => {
                    assert_ne!(stop_reason, "end_turn");
                    break;
                }
                _ => {}
            }
        }

        engines.stop("t1").await.unwrap();
    }

    #[tokio::test]
    async fn acp_mock_session_permission_and_turn_end() {
        let engines = registry();
        let approvals = engines.approvals.clone();
        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/engines/acp/mock_acp.py");
        engines
            .spawn(SpawnSpec {
                thread_id: "acp1".into(),
                kind: EngineKind::Cursor,
                cwd: std::env::temp_dir(),
                resume: None,
                override_argv: Some(vec!["python3".into(), script.display().to_string()]),
                auto_approve_readonly: false,
                auto_approve_all: false,
                read_only: false,
            })
            .await
            .unwrap();

        let mut events = engines.subscribe("acp1").unwrap();
        engines.prompt("acp1", "touch spike-ok.txt").unwrap();

        let permission_id = loop {
            let event = tokio::time::timeout(Duration::from_secs(5), events.recv())
                .await
                .unwrap()
                .unwrap();
            if let AgentEvent::Permission { id, .. } = event {
                break id;
            }
        };
        approvals
            .resolve(&permission_id, true, None)
            .await
            .unwrap();

        loop {
            let event = tokio::time::timeout(Duration::from_secs(5), events.recv())
                .await
                .unwrap()
                .unwrap();
            if let AgentEvent::TurnEnd { stop_reason, .. } = event {
                assert_eq!(stop_reason, "end_turn");
                break;
            }
        }
        engines.stop("acp1").await.unwrap();
    }

    #[test]
    fn acp_argv_for_codex_and_gemini() {
        assert_eq!(
            default_acp_argv(EngineKind::Gemini),
            vec!["gemini".to_string(), "--acp".into()]
        );
        assert_eq!(
            default_acp_argv(EngineKind::Codex),
            vec!["codex-acp".to_string()]
        );
        assert_eq!(
            default_acp_argv(EngineKind::Cursor),
            vec!["agent".to_string(), "acp".into()]
        );
    }

    fn seed_thread(store: &Store, thread_id: &str, session_id: Option<&str>) {
        use crate::store::types::{RepoRecord, ThreadRow, WorkspaceRow};
        store
            .upsert_repo(&RepoRecord {
                id: "r1".into(),
                path: "/tmp/app".into(),
                name: "app".into(),
                default_branch: Some("main".into()),
                setup_commands: String::new(),
                run_command: None,
            })
            .unwrap();
        store
            .upsert_workspace(&WorkspaceRow {
                id: "w1".into(),
                repo_id: "r1".into(),
                name: "Login".into(),
                branch: "feat".into(),
                worktree_path: "/tmp/wt".into(),
                status: "ready".into(),
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
        store
            .upsert_thread(&ThreadRow {
                id: thread_id.into(),
                workspace_id: "w1".into(),
                title: "Lead".into(),
                engine: "cursor".into(),
                session_id: session_id.map(str::to_string),
                status: "idle".into(),
                used_tokens: None,
                context_size: None,
                cost_usd: None,
                transcript_readonly: false,
            })
            .unwrap();
    }

    #[tokio::test]
    async fn acp_resumes_stored_session_and_emits_usage() {
        let engines = registry();
        seed_thread(&engines.store, "acp-resume", Some("sess-acp-1"));
        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/engines/acp/mock_acp.py");
        engines
            .spawn(SpawnSpec {
                thread_id: "acp-resume".into(),
                kind: EngineKind::Gemini,
                cwd: std::env::temp_dir(),
                resume: None,
                override_argv: Some(vec!["python3".into(), script.display().to_string()]),
                auto_approve_readonly: false,
                auto_approve_all: false,
                read_only: false,
            })
            .await
            .unwrap();

        let mut events = engines.subscribe("acp-resume").unwrap();
        engines.prompt("acp-resume", "continue").unwrap();
        let mut saw_usage = false;
        loop {
            let event = tokio::time::timeout(Duration::from_secs(5), events.recv())
                .await
                .unwrap()
                .unwrap();
            match event {
                AgentEvent::Usage { used_tokens, .. } => {
                    assert_eq!(used_tokens, 42);
                    saw_usage = true;
                }
                AgentEvent::Permission { id, .. } => {
                    engines
                        .approvals
                        .resolve(&id, true, None)
                        .await
                        .unwrap();
                }
                AgentEvent::TurnEnd { .. } => break,
                _ => {}
            }
        }
        assert!(saw_usage);
        engines.stop("acp-resume").await.unwrap();
    }

    #[tokio::test]
    async fn acp_failed_resume_seeds_next_prompt() {
        let engines = registry();
        seed_thread(&engines.store, "acp-miss", Some("missing-session"));
        engines
            .store
            .append_event("acp-miss", "message", "{\"text\":\"hello from before\"}")
            .unwrap();
        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/engines/acp/mock_acp.py");
        engines
            .spawn(SpawnSpec {
                thread_id: "acp-miss".into(),
                kind: EngineKind::Codex,
                cwd: std::env::temp_dir(),
                resume: None,
                override_argv: Some(vec!["python3".into(), script.display().to_string()]),
                auto_approve_readonly: false,
                auto_approve_all: false,
                read_only: false,
            })
            .await
            .unwrap();

        let mut events = engines.subscribe("acp-miss").unwrap();
        let mut saw_session = false;
        loop {
            let event = tokio::time::timeout(Duration::from_secs(5), events.recv())
                .await
                .unwrap()
                .unwrap();
            if matches!(event, AgentEvent::SessionStarted { .. }) {
                saw_session = true;
                break;
            }
        }
        assert!(saw_session);
        engines.prompt("acp-miss", "next").unwrap();
        loop {
            let event = tokio::time::timeout(Duration::from_secs(5), events.recv())
                .await
                .unwrap()
                .unwrap();
            match event {
                AgentEvent::Permission { id, .. } => {
                    engines
                        .approvals
                        .resolve(&id, true, None)
                        .await
                        .unwrap();
                }
                AgentEvent::TurnEnd { .. } => break,
                _ => {}
            }
        }
        let summary = engines.store.transcript_summary("acp-miss", 40).unwrap();
        assert!(summary.contains("hello from before") || !summary.is_empty());
        engines.stop("acp-miss").await.unwrap();
    }
}
