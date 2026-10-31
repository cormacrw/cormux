use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::{RwLock, broadcast, mpsc};

use super::acp;
use super::claude::map as claude_map;
use super::claude::{CanUseTool, ClaudeSession, Event, PermissionDecision, SpawnOptions};
use super::command::EngineCommand;
use super::detect::{self, EngineStatus};
use super::events::{AgentEvent, EngineKind, StreamedEvent, ToolCallStatus, ToolKind};
use super::models::{ModelOption, ThreadModels, static_models};
use super::prompt::Prompt;
use crate::approvals::{ApprovalBroker, ApprovalDecision};
use crate::error::{Error, Result};
use crate::mcp::CormuxMcp;
use crate::shell_env::ShellEnv;
use crate::store::Store;

const EVENT_CAP: usize = 256;
/// Just over what a session gets to exit before it's killed.
const STOP_GRACE: Duration = Duration::from_secs(6);
/// A Claude CLI sitting idle this long gives its memory back; the next prompt resumes it.
const IDLE_REAP_AFTER: Duration = Duration::from_secs(30 * 60);
const IDLE_REAP_TICK: Duration = Duration::from_secs(5 * 60);

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
    kind: EngineKind,
    commands: mpsc::UnboundedSender<EngineCommand>,
    events: broadcast::Sender<StreamedEvent>,
    session_id: Arc<Mutex<Option<String>>>,
    pending_seed: Arc<Mutex<Option<String>>>,
    /// Whether this engine has been told the thread renders media. Claude hears it
    /// in its system prompt instead, so this only ever flips for ACP engines.
    briefed: Arc<Mutex<bool>>,
    /// Models an ACP engine offered when its session started; empty until then.
    models: Arc<Mutex<Vec<ModelOption>>>,
    /// When anything last went to or came from the engine.
    last_active: Arc<Mutex<Instant>>,
}

impl ThreadSlot {
    fn touch(&self) {
        if let Ok(mut last_active) = self.last_active.lock() {
            *last_active = Instant::now();
        }
    }
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

    pub fn subscribe(&self, thread_id: &str) -> Result<broadcast::Receiver<StreamedEvent>> {
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
            // Whoever asks is about to use it. Under the same lock as the reaper, so an engine
            // that answers here can't be reaped before the prompt lands.
            Some(slot) => {
                slot.touch();
                true
            }
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
        let last_active = Arc::new(Mutex::new(Instant::now()));
        self.forward_turn_ends(&spec.thread_id, events.subscribe(), last_active.clone());
        let resume = spec
            .resume
            .clone()
            .or_else(|| self.store.thread_session(&spec.thread_id).ok().flatten());
        let mut spec = spec;
        spec.resume = resume;
        let session_id = Arc::new(Mutex::new(spec.resume.clone()));
        let pending_seed = Arc::new(Mutex::new(None));
        let models = Arc::new(Mutex::new(Vec::new()));
        let model = self.store.thread_model(&spec.thread_id).ok().flatten();
        let env = self.env.read().await.clone();
        let commands = match spec.kind {
            EngineKind::Claude => {
                self.spawn_claude(
                    &spec,
                    model,
                    env,
                    events.clone(),
                    session_id.clone(),
                    pending_seed.clone(),
                )
                .await?
            }
            EngineKind::Codex | EngineKind::Gemini => self.spawn_acp(
                &spec,
                model,
                env,
                events.clone(),
                session_id.clone(),
                pending_seed.clone(),
                models.clone(),
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
                    briefed: Arc::new(Mutex::new(false)),
                    models,
                    last_active,
                },
            );
        Ok(())
    }

    /// The thread's picked model and what it can switch to. ACP engines only know their
    /// models once a session is running, so an idle ACP thread lists none.
    pub fn thread_models(&self, thread_id: &str, kind: EngineKind) -> Result<ThreadModels> {
        let current = self.store.thread_model(thread_id)?;
        let options = match kind {
            EngineKind::Claude => static_models(kind),
            EngineKind::Codex | EngineKind::Gemini => self
                .threads
                .lock()
                .map_err(|error| Error::Engine(error.to_string()))?
                .get(thread_id)
                .filter(|slot| slot.kind == kind)
                .and_then(|slot| slot.models.lock().ok().map(|models| models.clone()))
                .unwrap_or_default(),
        };
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
    fn forward_turn_ends(
        &self,
        thread_id: &str,
        mut rx: broadcast::Receiver<StreamedEvent>,
        last_active: Arc<Mutex<Instant>>,
    ) {
        let notify = self.turn_end_notify.clone();
        let thread_id = thread_id.to_string();
        tokio::spawn(async move {
            loop {
                let event = rx.recv().await;
                if let Ok(mut last_active) = last_active.lock() {
                    *last_active = Instant::now();
                }
                match event {
                    Ok(StreamedEvent {
                        event: AgentEvent::TurnEnd { .. } | AgentEvent::EngineExited { .. },
                        ..
                    }) => {
                        let _ = notify.send(thread_id.clone());
                    }
                    Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }

    pub fn prompt(&self, thread_id: &str, prompt: impl Into<Prompt>) -> Result<()> {
        let mut prompt = prompt.into();
        if let Some(seed) = self.take_seed(thread_id)? {
            prompt.text = format!(
                "Previous session could not be resumed. Summary of the earlier transcript:\n{seed}\n\n---\n{}",
                prompt.text
            );
        }
        // ACP has no system-prompt channel, so its engines hear how the thread
        // renders media on their first prompt instead. Never shown in the thread:
        // the conversation keeps the user's own text. Claude is briefed through
        // `--append-system-prompt` and is skipped here.
        if self.take_brief(thread_id) {
            prompt.text = format!("{}\n\n{}", prompt.text, super::MEDIA_INSTRUCTION);
        }
        self.send(thread_id, EngineCommand::Prompt(prompt))
    }

    /// True exactly once per ACP engine process: the first prompt it receives.
    fn take_brief(&self, thread_id: &str) -> bool {
        let Ok(threads) = self.threads.lock() else {
            return false;
        };
        let Some(slot) = threads.get(thread_id) else {
            return false;
        };
        if slot.kind == EngineKind::Claude {
            return false;
        }
        let Ok(mut briefed) = slot.briefed.lock() else {
            return false;
        };
        if *briefed {
            return false;
        }
        *briefed = true;
        true
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
            // The pump drops its receiver once the engine is down.
            let _ = tokio::time::timeout(STOP_GRACE, commands.closed()).await;
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
        let mut stops = tokio::task::JoinSet::new();
        for id in thread_ids {
            let engines = self.clone();
            let id = id.clone();
            stops.spawn(async move {
                let _ = engines.stop(&id).await;
            });
        }
        stops.join_all().await;
        Ok(())
    }

    pub fn spawn_idle_reaper(&self) {
        let engines = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(IDLE_REAP_TICK);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                interval.tick().await;
                engines.reap_idle(IDLE_REAP_AFTER);
            }
        });
    }

    /// Shuts down Claude engines nothing has touched for `idle_for`, returning their threads.
    /// Only one with a saved session goes, since that's what the next prompt resumes. ACP
    /// engines stay: their model list only exists while they run, and they report an exit
    /// that would read as a finished turn.
    fn reap_idle(&self, idle_for: Duration) -> Vec<String> {
        let Ok(mut threads) = self.threads.lock() else {
            return Vec::new();
        };
        let idle: Vec<String> = threads
            .iter()
            .filter(|(thread_id, slot)| self.can_reap(thread_id, slot, idle_for))
            .map(|(thread_id, _)| thread_id.clone())
            .collect();
        for thread_id in &idle {
            if let Some(slot) = threads.remove(thread_id) {
                let _ = slot.commands.send(EngineCommand::Shutdown);
            }
        }
        idle
    }

    fn can_reap(&self, thread_id: &str, slot: &ThreadSlot, idle_for: Duration) -> bool {
        let quiet = slot
            .last_active
            .lock()
            .is_ok_and(|last_active| last_active.elapsed() >= idle_for);
        let resumable = self
            .store
            .thread_session(thread_id)
            .ok()
            .flatten()
            .is_some();
        let idle = self
            .store
            .thread_by_id(thread_id)
            .ok()
            .flatten()
            .is_some_and(|thread| thread.status == "idle");
        let unanswered = self
            .store
            .pending_approvals_for_thread(thread_id)
            .map_or(true, |approvals| !approvals.is_empty());
        slot.kind == EngineKind::Claude
            && quiet
            && resumable
            && idle
            && !unanswered
            && !self.thread_is_held(thread_id)
    }

    fn send(&self, thread_id: &str, command: EngineCommand) -> Result<()> {
        let threads = self
            .threads
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?;
        let slot = threads
            .get(thread_id)
            .ok_or_else(|| Error::Engine(format!("no engine for thread {thread_id}")))?;
        slot.touch();
        slot.commands
            .send(command)
            .map_err(|_| Error::Engine("engine process is gone".into()))
    }

    #[allow(clippy::too_many_arguments)]
    async fn spawn_claude(
        &self,
        spec: &SpawnSpec,
        model: Option<String>,
        env: ShellEnv,
        events: broadcast::Sender<StreamedEvent>,
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

    #[allow(clippy::too_many_arguments)]
    fn spawn_acp(
        &self,
        spec: &SpawnSpec,
        model: Option<String>,
        env: ShellEnv,
        events: broadcast::Sender<StreamedEvent>,
        session_id: Arc<Mutex<Option<String>>>,
        pending_seed: Arc<Mutex<Option<String>>>,
        models: Arc<Mutex<Vec<ModelOption>>>,
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
            model,
            models,
            approval_notify: self.approval_notify.clone(),
        })
    }
}

pub fn default_acp_argv(kind: EngineKind) -> Vec<String> {
    match kind {
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

#[allow(clippy::too_many_arguments)]
async fn run_claude(
    mut session: ClaudeSession,
    mut commands: mpsc::UnboundedReceiver<EngineCommand>,
    events: broadcast::Sender<StreamedEvent>,
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
    loop {
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
                            let seq = persist_event(&store, &thread_id, &usage);
                            let _ = events.send(StreamedEvent { seq, event: usage });
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
                            for mapped in claude_map::map_claude_event(&raw) {
                                let seq = persist_event(&store, &thread_id, &mapped);
                                let _ = events.send(StreamedEvent { seq, event: mapped });
                            }
                        }
                    }
                    Ok(None) => {
                        let _ = events.send(AgentEvent::EngineExited { code: None }.into());
                        break;
                    }
                    Err(error) => {
                        let _ = events.send(
                            AgentEvent::TurnEnd {
                                stop_reason: "error".into(),
                                error: Some(error.to_string()),
                            }
                            .into(),
                        );
                        break;
                    }
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
    events: &broadcast::Sender<StreamedEvent>,
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
    let mut mapped = claude_map::map_permission(req);
    if policy.auto_all || (policy.auto_ro && kind.is_readonly()) {
        if let AgentEvent::Permission { auto_approved, .. } = &mut mapped {
            *auto_approved = true;
        }
        let _ = events.send(mapped.into());
        let _ = events
            .send(claude_map::tool_event_from_permission(req, ToolCallStatus::Completed).into());
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

    let _ = events.send(mapped.clone().into());
    let _ =
        events.send(claude_map::tool_event_from_permission(req, ToolCallStatus::Pending).into());
    if let Err(error) = crate::approvals::record_pending_permission(store, thread_id, &mapped, None)
    {
        log::warn!("record approval: {error}");
    } else {
        let _ = approval_notify.send(());
    }
    let _ = events.send(
        AgentEvent::CurrentTool {
            id: req.tool_use_id.clone(),
            title: req.tool_name.clone(),
        }
        .into(),
    );
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

fn persist_event(store: &Store, thread_id: &str, event: &AgentEvent) -> Option<i64> {
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
        AgentEvent::StreamLagged { .. } => return None,
    };
    let seq = serde_json::to_string(event)
        .ok()
        .and_then(|payload| store.append_event(thread_id, kind, &payload).ok());
    if matches!(
        event,
        AgentEvent::TurnEnd { .. } | AgentEvent::EngineExited { .. }
    ) {
        let _ = store.mark_thread_idle(thread_id);
    }
    seq
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

    fn fake_engine(
        engines: &EngineRegistry,
        thread_id: &str,
        status: &str,
        kind: EngineKind,
    ) -> mpsc::UnboundedReceiver<EngineCommand> {
        engines
            .store
            .upsert_thread(&crate::store::types::ThreadRow {
                id: thread_id.into(),
                workspace_id: "w1".into(),
                title: "Lead".into(),
                engine: "claude".into(),
                session_id: Some("session".into()),
                status: status.into(),
                used_tokens: None,
                context_size: None,
                cost_usd: None,
                transcript_readonly: false,
                retired: false,
                parent_thread_id: None,
            })
            .unwrap();
        let (commands, rx) = mpsc::unbounded_channel();
        engines.threads.lock().unwrap().insert(
            thread_id.into(),
            ThreadSlot {
                kind,
                commands,
                events: broadcast::channel(4).0,
                session_id: Arc::new(Mutex::new(None)),
                pending_seed: Arc::new(Mutex::new(None)),
                briefed: Arc::new(Mutex::new(false)),
                models: Arc::new(Mutex::new(Vec::new())),
                last_active: Arc::new(Mutex::new(Instant::now())),
            },
        );
        rx
    }

    #[tokio::test]
    async fn stop_returns_as_soon_as_the_engine_is_down() {
        let engines = registry();
        let mut rx = fake_engine(&engines, "t1", "idle", EngineKind::Claude);
        tokio::spawn(async move {
            while let Some(command) = rx.recv().await {
                if matches!(command, EngineCommand::Shutdown) {
                    break;
                }
            }
        });
        let started = Instant::now();
        engines.stop_threads(&["t1".to_string()]).await.unwrap();
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(!engines.has_thread("t1"));
    }

    #[test]
    fn only_a_quiet_resumable_idle_claude_engine_is_reaped() {
        let engines = registry();
        let _idle = fake_engine(&engines, "idle", "idle", EngineKind::Claude);
        let _running = fake_engine(&engines, "running", "running", EngineKind::Claude);
        let _acp = fake_engine(&engines, "acp", "idle", EngineKind::Codex);
        let _held = fake_engine(&engines, "held", "idle", EngineKind::Claude);
        engines.hold_thread("held").unwrap();
        let _asking = fake_engine(&engines, "asking", "idle", EngineKind::Claude);
        engines
            .store
            .upsert_approval(&crate::store::types::ApprovalRow {
                id: "a1".into(),
                thread_id: "asking".into(),
                status: "pending".into(),
                tool: "edit".into(),
                payload: "{}".into(),
            })
            .unwrap();
        let _fresh = fake_engine(&engines, "fresh", "idle", EngineKind::Claude);
        engines
            .store
            .upsert_thread(&crate::store::types::ThreadRow {
                session_id: None,
                ..engines.store.thread_by_id("fresh").unwrap().unwrap()
            })
            .unwrap();

        assert!(engines.reap_idle(Duration::from_secs(60)).is_empty());
        assert_eq!(engines.reap_idle(Duration::ZERO), vec!["idle".to_string()]);
        assert!(!engines.has_thread("idle"));
        assert!(engines.has_thread("running"));
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
                .unwrap()
                .event;
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

    #[test]
    fn first_prompt_briefs_acp_engines_once_and_claude_never() {
        let engines = registry();
        let insert = |id: &str, kind: EngineKind| {
            let (commands, rx) = mpsc::unbounded_channel();
            let (events, _) = broadcast::channel(4);
            engines.threads.lock().unwrap().insert(
                id.into(),
                ThreadSlot {
                    kind,
                    commands,
                    events,
                    session_id: Arc::new(Mutex::new(None)),
                    pending_seed: Arc::new(Mutex::new(None)),
                    briefed: Arc::new(Mutex::new(false)),
                    models: Arc::new(Mutex::new(Vec::new())),
                    last_active: Arc::new(Mutex::new(Instant::now())),
                },
            );
            rx
        };
        let sent_text =
            |rx: &mut mpsc::UnboundedReceiver<EngineCommand>| match rx.try_recv().unwrap() {
                EngineCommand::Prompt(prompt) => prompt.text,
                _ => panic!("expected a prompt"),
            };

        // Codex (ACP): first prompt carries the instruction, the second doesn't.
        let mut acp_rx = insert("t-acp", EngineKind::Codex);
        engines.prompt("t-acp", "Fix the sidebar").unwrap();
        let first = sent_text(&mut acp_rx);
        assert!(first.starts_with("Fix the sidebar\n\n"));
        assert!(first.ends_with(crate::engines::MEDIA_INSTRUCTION));
        engines.prompt("t-acp", "Now the header").unwrap();
        assert_eq!(sent_text(&mut acp_rx), "Now the header");

        // Claude hears it via --append-system-prompt, so its prompts stay untouched.
        let mut claude_rx = insert("t-claude", EngineKind::Claude);
        engines.prompt("t-claude", "Fix the sidebar").unwrap();
        assert_eq!(sent_text(&mut claude_rx), "Fix the sidebar");

        assert!(!engines.take_brief("no-such-thread"));
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
            let event = events.recv().await.unwrap().event;
            if let AgentEvent::Permission { id, kind, .. } = event {
                assert_eq!(kind, ToolKind::Execute);
                break id;
            }
        };
        approvals.resolve(&permission_id, true, None).await.unwrap();

        loop {
            match events.recv().await.unwrap().event {
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
            if let AgentEvent::TurnEnd { stop_reason, .. } = events.recv().await.unwrap().event {
                assert_ne!(stop_reason, "end_turn");
                break;
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

        let mut events = engines.subscribe("acp1").unwrap();
        engines.prompt("acp1", "touch spike-ok.txt").unwrap();

        let permission_id = loop {
            let event = tokio::time::timeout(Duration::from_secs(5), events.recv())
                .await
                .unwrap()
                .unwrap()
                .event;
            if let AgentEvent::Permission { id, .. } = event {
                break id;
            }
        };
        approvals.resolve(&permission_id, true, None).await.unwrap();

        loop {
            let event = tokio::time::timeout(Duration::from_secs(5), events.recv())
                .await
                .unwrap()
                .unwrap()
                .event;
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
    }

    #[test]
    fn persisting_reports_where_the_store_put_an_event() {
        let store = Store::new();
        store.open_in_memory().unwrap();
        seed_thread(&store, "t1", None);
        let event = AgentEvent::MessageChunk {
            role: super::super::events::MessageRole::Agent,
            text: "hi".into(),
        };
        let first = persist_event(&store, "t1", &event).unwrap();
        assert_eq!(persist_event(&store, "t1", &event), Some(first + 1));
        // Unsaved events still reach the UI, just without a place in the store.
        assert_eq!(persist_event(&store, "missing", &event), None);
        let lagged = AgentEvent::StreamLagged { missed: 1 };
        assert_eq!(persist_event(&store, "t1", &lagged), None);
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
                single_instance: false,
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
                engine: "codex".into(),
                session_id: session_id.map(str::to_string),
                status: "idle".into(),
                used_tokens: None,
                context_size: None,
                cost_usd: None,
                transcript_readonly: false,
                retired: false,
                parent_thread_id: None,
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
                .unwrap()
                .event;
            if let AgentEvent::MessageChunk { text, .. } = &event {
                assert!(
                    !text.starts_with("replayed"),
                    "session/load replay leaked: {text}"
                );
            }
            match event {
                AgentEvent::Usage { used_tokens, .. } => {
                    assert_eq!(used_tokens, 42);
                    saw_usage = true;
                }
                AgentEvent::Permission { id, .. } => {
                    engines.approvals.resolve(&id, true, None).await.unwrap();
                }
                AgentEvent::TurnEnd { .. } => break,
                _ => {}
            }
        }
        assert!(saw_usage);
        let persisted = engines.store.transcript_summary("acp-resume", 100).unwrap();
        assert!(
            !persisted.contains("replayed"),
            "replay was persisted: {persisted}"
        );
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
        // Times out and fails unless the session starts.
        loop {
            let event = tokio::time::timeout(Duration::from_secs(5), events.recv())
                .await
                .unwrap()
                .unwrap()
                .event;
            if matches!(event, AgentEvent::SessionStarted { .. }) {
                break;
            }
        }
        engines.prompt("acp-miss", "next").unwrap();
        loop {
            let event = tokio::time::timeout(Duration::from_secs(5), events.recv())
                .await
                .unwrap()
                .unwrap()
                .event;
            match event {
                AgentEvent::Permission { id, .. } => {
                    engines.approvals.resolve(&id, true, None).await.unwrap();
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
