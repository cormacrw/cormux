use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::{RwLock, broadcast, mpsc};

use super::claude::map as claude_map;
use super::claude::turn_gate::TurnGate;
use super::claude::{CanUseTool, ClaudeSession, Event, PermissionDecision, SpawnOptions};
use super::command::EngineCommand;
use super::detect::{self, EngineStatus};
use super::events::{AgentEvent, EngineKind, ToolCallStatus, ToolKind};
use super::models::{ThreadModels, static_models};
use super::prompt::Prompt;
use crate::approvals::{ApprovalBroker, ApprovalDecision};
use crate::error::{Error, Result};
use crate::mcp::CormuxMcp;
use crate::shell_env::ShellEnv;
use crate::store::Store;

const EVENT_CAP: usize = 256;
const STOP_GRACE: Duration = Duration::from_secs(5);

#[derive(Default)]
struct HoldState {
    paused: bool,
    queued: Vec<Prompt>,
}

#[derive(Clone)]
pub struct EngineRegistry {
    env: Arc<RwLock<ShellEnv>>,
    approvals: Arc<ApprovalBroker>,
    pub(crate) store: Store,
    mcp: CormuxMcp,
    approval_notify: tokio::sync::broadcast::Sender<()>,
    /// Thread ids whose turn just ended (or whose engine exited).
    turn_end_notify: tokio::sync::broadcast::Sender<String>,
    threads: Arc<Mutex<HashMap<String, ThreadSlot>>>,
    holds: Arc<Mutex<HashMap<String, HoldState>>>,
}

struct ThreadSlot {
    commands: mpsc::UnboundedSender<EngineCommand>,
    events: broadcast::Sender<AgentEvent>,
    session_id: Arc<Mutex<Option<String>>>,
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
        mcp: CormuxMcp,
        approval_notify: tokio::sync::broadcast::Sender<()>,
        turn_end_notify: tokio::sync::broadcast::Sender<String>,
    ) -> Self {
        Self {
            env,
            approvals,
            store,
            mcp,
            approval_notify,
            turn_end_notify,
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

    /// Whether a live engine serves the thread. An engine whose pump has ended (the CLI
    /// exited or its output broke) is dropped here, so the caller spawns a fresh one that
    /// resumes the session instead of prompting into nothing.
    pub fn has_thread(&self, thread_id: &str) -> bool {
        let Ok(mut threads) = self.threads.lock() else {
            return false;
        };
        match threads.get(thread_id) {
            Some(slot) if slot.commands.is_closed() => {
                threads.remove(thread_id);
                false
            }
            Some(_) => true,
            None => false,
        }
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
        self.forward_turn_ends(&spec.thread_id, events.subscribe());
        let resume = spec
            .resume
            .clone()
            .or_else(|| self.store.thread_session(&spec.thread_id).ok().flatten());
        let mut spec = spec;
        spec.resume = resume;
        let session_id = Arc::new(Mutex::new(spec.resume.clone()));
        let model = self.store.thread_model(&spec.thread_id).ok().flatten();
        let env = self.env.read().await.clone();
        let commands = self
            .spawn_claude(&spec, model, env, events.clone(), session_id.clone())
            .await?;

        self.threads
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?
            .insert(
                spec.thread_id.clone(),
                ThreadSlot {
                    commands,
                    events,
                    session_id,
                },
            );
        Ok(())
    }

    /// The thread's picked model and what it can switch to.
    pub fn thread_models(&self, thread_id: &str, kind: EngineKind) -> Result<ThreadModels> {
        let current = self.store.thread_model(thread_id)?;
        let options = static_models(kind);
        Ok(ThreadModels { current, options })
    }

    /// Saves the pick for future spawns and switches a live engine over now.
    pub fn set_thread_model(&self, thread_id: &str, model: Option<String>) -> Result<()> {
        self.store.set_thread_model(thread_id, model.as_deref())?;
        if self.has_thread(thread_id) {
            self.send(thread_id, EngineCommand::SetModel(model))?;
        }
        Ok(())
    }

    /// Every engine ends a turn through its event stream, so this is the one place to hear it.
    fn forward_turn_ends(&self, thread_id: &str, mut rx: broadcast::Receiver<AgentEvent>) {
        let notify = self.turn_end_notify.clone();
        let thread_id = thread_id.to_string();
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(AgentEvent::TurnEnd { .. } | AgentEvent::EngineExited { .. }) => {
                        let _ = notify.send(thread_id.clone());
                    }
                    Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }

    pub fn prompt(&self, thread_id: &str, prompt: impl Into<Prompt>) -> Result<()> {
        self.send(thread_id, EngineCommand::Prompt(prompt.into()))
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
        holds.entry(thread_id.to_string()).or_default().paused = true;
        Ok(())
    }

    pub fn release_thread(&self, thread_id: &str) -> Result<Option<Prompt>> {
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
        Ok(Some(Prompt::merge(queued)))
    }

    pub fn enqueue_while_held(&self, thread_id: &str, prompt: Prompt) -> Result<()> {
        let mut holds = self
            .holds
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?;
        holds
            .entry(thread_id.to_string())
            .or_default()
            .queued
            .push(prompt);
        Ok(())
    }

    pub fn submit_prompt(
        &self,
        thread_id: &str,
        prompt: impl Into<Prompt>,
        held: bool,
    ) -> Result<()> {
        let prompt = prompt.into();
        if held || self.thread_is_held(thread_id) {
            return self.enqueue_while_held(thread_id, prompt);
        }
        self.prompt(thread_id, prompt)
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

    /// Shuts the engine down without waiting for it, and drops any paused turn and queued prompts.
    /// The next prompt spawns a new engine.
    pub fn discard(&self, thread_id: &str) -> Result<()> {
        let slot = self
            .threads
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?
            .remove(thread_id);
        if let Some(slot) = slot {
            let _ = slot.commands.send(EngineCommand::Shutdown);
        }
        self.holds
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?
            .remove(thread_id);
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
        model: Option<String>,
        env: ShellEnv,
        events: broadcast::Sender<AgentEvent>,
        session_id: Arc<Mutex<Option<String>>>,
    ) -> Result<mpsc::UnboundedSender<EngineCommand>> {
        let (binary, extra_args, use_default_args) = if let Some(argv) = &spec.override_argv {
            let mut argv = argv.clone();
            let binary = PathBuf::from(argv.remove(0));
            (binary, argv, false)
        } else {
            let binary = detect::resolve_binary(&env, spec.kind.binaries())
                .unwrap_or_else(|| PathBuf::from("claude"));
            // Nothing in the app could take a session back out of plan mode.
            let extra = vec!["--disallowedTools".into(), "EnterPlanMode".into()];
            (binary, extra, true)
        };

        let mut options = SpawnOptions {
            binary,
            cwd: Some(spec.cwd.clone()),
            resume: spec.resume.clone(),
            model,
            extra_args,
            use_default_args,
            // A read-only scratch has no workspace to start threads from.
            orchestrate: use_default_args && !spec.read_only,
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
        let mcp = self.mcp.clone();
        tokio::spawn(async move {
            run_claude(
                session,
                rx,
                events,
                approvals,
                store,
                mcp,
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
}

#[allow(clippy::too_many_arguments)]
async fn run_claude(
    mut session: ClaudeSession,
    mut commands: mpsc::UnboundedReceiver<EngineCommand>,
    events: broadcast::Sender<AgentEvent>,
    approvals: Arc<ApprovalBroker>,
    store: Store,
    mcp: CormuxMcp,
    thread_id: String,
    session_id: Arc<Mutex<Option<String>>>,
    policy: ClaudePolicy,
    approval_notify: tokio::sync::broadcast::Sender<()>,
    // Where leaving plan mode goes back to; a session that starts outside plan mode updates it.
) {
    let (answer_tx, mut answer_rx) = mpsc::unbounded_channel::<ClaudeAnswer>();
    // Tool calls run off the pump, which has to keep reading while they work.
    let (mcp_tx, mut mcp_rx) = mpsc::unbounded_channel::<(String, serde_json::Value)>();
    // Claude's running cost for this process, so each turn adds only what it cost.
    let mut process_cost = 0.0_f64;
    let mut gate = TurnGate::default();
    loop {
        let release_at = gate.release_at();
        tokio::select! {
            command = commands.recv() => {
                match command {
                    Some(EngineCommand::Prompt(prompt)) => {
                        if let Err(error) = session.send_prompt(&prompt).await {
                            log::warn!("claude prompt: {error}");
                        }
                    }
                    Some(EngineCommand::Cancel) => {
                        let _ = session.interrupt().await;
                        for task_id in gate.running_agents() {
                            if let Err(error) = session.stop_task(task_id).await {
                                log::warn!("claude stop task: {error}");
                            }
                        }
                        if let Some(held) = gate.release() {
                            persist_event(&store, &thread_id, &held);
                            let _ = events.send(held);
                        }
                    }
                    Some(EngineCommand::SetModel(model)) => {
                        if let Err(error) = session.set_model(model).await {
                            log::warn!("claude set model: {error}");
                        }
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
                        if let Event::Init { session_id: id, .. } = &raw {
                            *session_id.lock().unwrap() = Some(id.clone());
                            let _ = store.set_thread_session(&thread_id, id);
                        }
                        if let Some(AgentEvent::Usage {
                            used_tokens,
                            context_size,
                            cost_usd,
                        }) = claude_map::usage_from_result(&raw)
                        {
                            let saved = store
                                .thread_by_id(&thread_id)
                                .ok()
                                .flatten()
                                .and_then(|thread| thread.cost_usd);
                            let cost_usd = match cost_usd {
                                Some(process_total) => {
                                    let (thread_total, seen) = claude_map::add_turn_cost(
                                        saved.unwrap_or(0.0),
                                        process_cost,
                                        process_total,
                                    );
                                    process_cost = seen;
                                    Some(thread_total)
                                }
                                None => saved,
                            };
                            let usage = AgentEvent::Usage {
                                used_tokens,
                                context_size,
                                cost_usd,
                            };
                            persist_event(&store, &thread_id, &usage);
                            let _ = events.send(usage);
                            let _ = store.set_thread_usage(
                                &thread_id,
                                used_tokens,
                                context_size,
                                cost_usd,
                            );
                        }
                        if let Event::McpMessage { request_id, message, server_name } = raw {
                            let reply = mcp_tx.clone();
                            let mcp = mcp.clone();
                            let thread_id = thread_id.clone();
                            tokio::spawn(async move {
                                let answer = if server_name == crate::mcp::SERVER_NAME {
                                    mcp.handle_message(&thread_id, &message).await
                                } else {
                                    crate::mcp::unknown_server_reply(&message, &server_name)
                                };
                                let _ = reply.send((request_id, answer));
                            });
                        } else if let Event::CanUseTool(req) = &raw {
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
                        } else {
                            gate.observe(&raw);
                            for mapped in claude_map::map_claude_event(&raw) {
                                if let Some(mapped) = gate.pass(mapped) {
                                    persist_event(&store, &thread_id, &mapped);
                                    let _ = events.send(mapped);
                                }
                            }
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
            _ = tokio::time::sleep_until(release_at.unwrap_or_else(tokio::time::Instant::now)), if release_at.is_some() => {
                if let Some(held) = gate.release() {
                    persist_event(&store, &thread_id, &held);
                    let _ = events.send(held);
                }
            }
            Some((request_id, answer)) = mcp_rx.recv() => {
                if let Err(error) = session.answer_mcp(&request_id, answer).await {
                    log::warn!("claude mcp answer: {error}");
                }
            }
            Some(answer) = answer_rx.recv() => {
                let id = answer.request_id.clone();
                let _ = session.answer_tool(&id, permission_decision(answer)).await;
            }
        }
    }
}

/// The user's decision on a tool Claude asked to use, on its way back to the CLI.
struct ClaudeAnswer {
    request_id: String,
    decision: ApprovalDecision,
    input: serde_json::Value,
}

fn permission_decision(answer: ClaudeAnswer) -> PermissionDecision {
    match answer.decision {
        ApprovalDecision::Approved => PermissionDecision::Allow {
            updated_input: answer.input,
            updated_permissions: None,
        },
        ApprovalDecision::Denied { message } => PermissionDecision::Deny { message },
        ApprovalDecision::Answered { answers } => {
            let mut updated_input = answer.input;
            updated_input["answers"] = serde_json::json!(answers);
            PermissionDecision::Allow {
                updated_input,
                updated_permissions: None,
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

#[allow(clippy::too_many_arguments)]
async fn handle_claude_permission(
    session: &mut ClaudeSession,
    req: &CanUseTool,
    events: &broadcast::Sender<AgentEvent>,
    approvals: &Arc<ApprovalBroker>,
    store: &Store,
    thread_id: &str,
    approval_notify: &tokio::sync::broadcast::Sender<()>,
    policy: ClaudePolicy,
    answer_tx: mpsc::UnboundedSender<ClaudeAnswer>,
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
    // Only the user can answer a question, so no auto-approve setting applies to one.
    let questions = if req.tool_name == "AskUserQuestion" {
        crate::approvals::questions_from_input(&req.input)
    } else {
        Vec::new()
    };
    let mut mapped = claude_map::map_permission(req);
    if questions.is_empty() && (policy.auto_all || (policy.auto_ro && kind.is_readonly())) {
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
                    updated_permissions: None,
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
    let recorded = if questions.is_empty() {
        crate::approvals::record_pending_permission(store, thread_id, &mapped)
    } else {
        crate::approvals::record_pending_question(store, thread_id, &mapped, questions)
    };
    if let Err(error) = recorded {
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
        let _ = answer_tx.send(ClaudeAnswer {
            request_id: id,
            decision,
            input,
        });
    });
}

fn persist_event(store: &Store, thread_id: &str, event: &AgentEvent) {
    let kind = match event {
        AgentEvent::SessionStarted { .. } => "session",
        AgentEvent::MessageChunk { .. } => "message",
        AgentEvent::ToolCall { .. } => "tool",
        AgentEvent::Plan { .. } => "plan",
        AgentEvent::Permission { .. } => "permission",
        AgentEvent::ToolCallUpdate { .. } => "tool_update",
        AgentEvent::CurrentTool { .. } => "current_tool",
        AgentEvent::Usage { .. } => "usage",
        AgentEvent::TurnEnd { .. } => "turn_end",
        AgentEvent::EngineExited { .. } => "exit",
    };
    if let Ok(payload) = serde_json::to_string(event) {
        let _ = store.append_event(thread_id, kind, &payload);
    }
    if matches!(
        event,
        AgentEvent::TurnEnd { .. } | AgentEvent::EngineExited { .. }
    ) {
        let _ = store.mark_thread_idle(thread_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_mcp(store: Store) -> CormuxMcp {
        let env = Arc::new(RwLock::new(ShellEnv::new()));
        CormuxMcp::new(
            store,
            crate::process::ProcessSupervisor::new(env),
            Arc::new(ApprovalBroker::new()),
            crate::app::WorkspaceAppService::new(),
        )
    }
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
        EngineRegistry::new(
            env,
            approvals,
            store.clone(),
            test_mcp(store),
            approval_notify,
            broadcast::channel(8).0,
        )
    }

    /// Starts a real Haiku session and has it launch one background agent, returning the
    /// session's events once they begin.
    async fn live_background_agent(
        thread_id: &str,
    ) -> (EngineRegistry, broadcast::Receiver<AgentEvent>) {
        let store = Store::new();
        store.open_in_memory().unwrap();
        store
            .upsert_thread(&crate::store::types::ThreadRow {
                id: thread_id.into(),
                workspace_id: "live-workspace".into(),
                title: "Lead".into(),
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
        store.set_thread_model(thread_id, Some("haiku")).unwrap();
        let engines = EngineRegistry::new(
            Arc::new(RwLock::new(ShellEnv::new())),
            Arc::new(ApprovalBroker::new()),
            store.clone(),
            test_mcp(store),
            broadcast::channel(4).0,
            broadcast::channel(8).0,
        );
        engines
            .spawn(SpawnSpec {
                thread_id: thread_id.into(),
                kind: EngineKind::Claude,
                cwd: std::env::temp_dir(),
                resume: None,
                override_argv: None,
                auto_approve_readonly: false,
                auto_approve_all: true,
                read_only: false,
            })
            .await
            .unwrap();
        let events = engines.subscribe(thread_id).unwrap();
        engines
            .prompt(
                thread_id,
                "Use the Agent tool with run_in_background set to true and subagent_type \
                 general-purpose. Its prompt: run the Bash command \
                 `python3 -c \"import time; time.sleep(15)\"` in the foreground, then reply ok. \
                 Once it is launched, end your turn with the single word launched.",
            )
            .unwrap();
        (engines, events)
    }

    async fn next_live_event(events: &mut broadcast::Receiver<AgentEvent>) -> AgentEvent {
        tokio::time::timeout(Duration::from_secs(120), events.recv())
            .await
            .expect("an event within 120s")
            .unwrap()
    }

    /// Needs a signed-in `claude` on PATH, and spends a few small Haiku turns.
    #[tokio::test]
    #[ignore]
    async fn live_claude_holds_the_turn_for_a_background_agent() {
        let (engines, mut events) = live_background_agent("live-bg").await;
        let mut agent = None;
        let mut agent_finished = false;
        loop {
            match next_live_event(&mut events).await {
                AgentEvent::ToolCall { id, name, .. } if name.as_deref() == Some("Agent") => {
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
        engines.stop("live-bg").await.unwrap();
        assert!(agent.is_some(), "never launched an agent");
        assert!(agent_finished, "turn ended before its agent did");
    }

    /// Needs a signed-in `claude` on PATH, and spends a few small Haiku turns.
    #[tokio::test]
    #[ignore]
    async fn live_claude_stop_reaches_a_background_agent() {
        let (engines, mut events) = live_background_agent("live-stop").await;
        let mut agent = None;
        // The agent's first step means it is under way and the main turn is wrapping up.
        loop {
            match next_live_event(&mut events).await {
                AgentEvent::ToolCall { id, name, .. } if name.as_deref() == Some("Agent") => {
                    agent = Some(id);
                }
                AgentEvent::ToolCall { parent_id, .. } if parent_id.is_some() => break,
                AgentEvent::TurnEnd { .. } => panic!("turn ended before its agent did"),
                _ => {}
            }
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
        engines.cancel("live-stop").unwrap();
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
        engines.stop("live-stop").await.unwrap();
        assert!(stopped.is_ok(), "the agent kept running after Stop");
    }

    /// Needs a signed-in `claude` on PATH, and spends one small Haiku turn.
    #[tokio::test]
    #[ignore]
    async fn live_claude_reaches_the_cormux_tools() {
        let store = Store::new();
        store.open_in_memory().unwrap();
        store
            .upsert_thread(&crate::store::types::ThreadRow {
                id: "live".into(),
                workspace_id: "live-workspace".into(),
                title: "Lead".into(),
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
        store.set_thread_model("live", Some("haiku")).unwrap();
        let engines = EngineRegistry::new(
            Arc::new(RwLock::new(ShellEnv::new())),
            Arc::new(ApprovalBroker::new()),
            store.clone(),
            test_mcp(store.clone()),
            broadcast::channel(4).0,
            broadcast::channel(8).0,
        );
        engines
            .spawn(SpawnSpec {
                thread_id: "live".into(),
                kind: EngineKind::Claude,
                cwd: std::env::temp_dir(),
                resume: None,
                override_argv: None,
                auto_approve_readonly: false,
                auto_approve_all: false,
                read_only: false,
            })
            .await
            .unwrap();
        let mut events = engines.subscribe("live").unwrap();
        engines
            .prompt(
                "live",
                "Call mcp__cormux__get_thread with id \"nonexistent\", then quote its error.",
            )
            .unwrap();
        let mut reply = String::new();
        let mut used_tool = false;
        loop {
            let event = tokio::time::timeout(Duration::from_secs(90), events.recv())
                .await
                .expect("the turn timed out")
                .unwrap();
            match event {
                AgentEvent::ToolCall {
                    name: Some(name), ..
                } => {
                    used_tool |= name == "mcp__cormux__get_thread";
                }
                AgentEvent::Permission { .. } => panic!("a read tool asked for approval"),
                AgentEvent::MessageChunk { text, .. } => reply.push_str(&text),
                AgentEvent::TurnEnd { .. } => break,
                _ => {}
            }
        }
        engines.stop("live").await.unwrap();
        assert!(used_tool, "never called the tool: {reply}");
        assert!(reply.contains("unknown thread"), "{reply}");
    }

    #[tokio::test]
    async fn questions_wait_for_answers_even_with_auto_approve_on() {
        let store = Store::new();
        store.open_in_memory().unwrap();
        store
            .upsert_thread(&crate::store::types::ThreadRow {
                id: "t-ask".into(),
                workspace_id: "w1".into(),
                title: "Lead".into(),
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
        let approvals = Arc::new(ApprovalBroker::new());
        let engines = EngineRegistry::new(
            Arc::new(RwLock::new(ShellEnv::new())),
            approvals.clone(),
            store.clone(),
            test_mcp(store.clone()),
            broadcast::channel(4).0,
            broadcast::channel(4).0,
        );
        engines
            .spawn(SpawnSpec {
                thread_id: "t-ask".into(),
                kind: EngineKind::Claude,
                cwd: std::env::temp_dir(),
                resume: None,
                override_argv: Some(mock_claude_argv()),
                auto_approve_readonly: true,
                auto_approve_all: true,
                read_only: false,
            })
            .await
            .unwrap();

        let mut events = engines.subscribe("t-ask").unwrap();
        engines.prompt("t-ask", "ask me something").unwrap();
        let id = loop {
            if let AgentEvent::Permission {
                id, auto_approved, ..
            } = events.recv().await.unwrap()
            {
                assert!(!auto_approved);
                break id;
            }
        };

        // The broker registers just after the event goes out.
        while approvals.pending_count() == 0 {
            tokio::task::yield_now().await;
        }
        let row = store.get_approval(&id).unwrap().unwrap();
        assert_eq!(row.tool, crate::approvals::QUESTION_TOOL);
        assert!(row.payload.contains("Which colour?"));

        let answers = std::collections::BTreeMap::from([(
            "Which colour?".to_string(),
            "Green, my own".to_string(),
        )]);
        approvals
            .complete(&id, ApprovalDecision::Answered { answers })
            .unwrap();

        // The mock echoes the decision back as a message.
        let mut reply = String::new();
        loop {
            match events.recv().await.unwrap() {
                AgentEvent::MessageChunk { text, .. } => reply.push_str(&text),
                AgentEvent::TurnEnd { .. } => break,
                AgentEvent::EngineExited { .. } => panic!("exited early"),
                _ => {}
            }
        }
        engines.stop("t-ask").await.unwrap();
        let echoed: serde_json::Value = serde_json::from_str(&reply).unwrap();
        assert_eq!(echoed["behavior"], "allow");
        assert_eq!(
            echoed["updatedInput"]["answers"]["Which colour?"],
            "Green, my own"
        );
        assert_eq!(
            echoed["updatedInput"]["questions"][0]["header"], "Colour",
            "the CLI needs the original input back too"
        );
    }

    #[tokio::test]
    async fn turn_end_is_forwarded_with_the_thread_id() {
        let env = Arc::new(RwLock::new(ShellEnv::new()));
        let store = Store::new();
        store.open_in_memory().unwrap();
        let (turn_end_notify, mut turn_ends) = broadcast::channel(8);
        let engines = EngineRegistry::new(
            env,
            Arc::new(ApprovalBroker::new()),
            store.clone(),
            test_mcp(store),
            broadcast::channel(4).0,
            turn_end_notify,
        );
        engines
            .spawn(SpawnSpec {
                thread_id: "t-idle".into(),
                kind: EngineKind::Claude,
                cwd: std::env::temp_dir(),
                resume: None,
                override_argv: Some(mock_claude_argv()),
                auto_approve_readonly: true,
                auto_approve_all: true,
                read_only: false,
            })
            .await
            .unwrap();
        engines
            .prompt("t-idle", "first: run echo spike-ok")
            .unwrap();
        let thread_id = tokio::time::timeout(std::time::Duration::from_secs(10), turn_ends.recv())
            .await
            .expect("turn end within 10s")
            .unwrap();
        assert_eq!(thread_id, "t-idle");
    }

    #[tokio::test]
    async fn a_turn_ends_only_after_its_background_agents_do() {
        let env = Arc::new(RwLock::new(ShellEnv::new()));
        let store = Store::new();
        store.open_in_memory().unwrap();
        let (turn_end_notify, mut turn_ends) = broadcast::channel(8);
        let engines = EngineRegistry::new(
            env,
            Arc::new(ApprovalBroker::new()),
            store.clone(),
            test_mcp(store),
            broadcast::channel(4).0,
            turn_end_notify,
        );
        engines
            .spawn(SpawnSpec {
                thread_id: "t-bg".into(),
                kind: EngineKind::Claude,
                cwd: std::env::temp_dir(),
                resume: None,
                override_argv: Some(mock_claude_argv()),
                auto_approve_readonly: true,
                auto_approve_all: true,
                read_only: false,
            })
            .await
            .unwrap();
        engines.prompt("t-bg", "background agent").unwrap();
        // The mock's first turn finishes at once; its agent takes a moment longer.
        let early =
            tokio::time::timeout(std::time::Duration::from_millis(300), turn_ends.recv()).await;
        assert!(early.is_err(), "turn ended with an agent still running");
        let thread_id = tokio::time::timeout(std::time::Duration::from_secs(10), turn_ends.recv())
            .await
            .expect("turn end within 10s")
            .unwrap();
        assert_eq!(thread_id, "t-bg");
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
        approvals.resolve(&permission_id, true, None).await.unwrap();

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
            if let AgentEvent::TurnEnd { stop_reason, .. } = events.recv().await.unwrap() {
                assert_ne!(stop_reason, "end_turn");
                break;
            }
        }

        engines.stop("t1").await.unwrap();
    }
}
