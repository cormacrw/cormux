//! Each thread's live engine: launching one in the thread's worktree when a prompt needs it,
//! routing what it reports (events into the transcript, permission requests to approvals, tool
//! calls to the host's tools), adding up its cost, publishing each turn's end once, and shutting
//! down engines that sit idle.

#[cfg(test)]
pub mod fake;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, mpsc};

use crate::approvals::{ApprovalBroker, ApprovalDecision};
use crate::engines::prompt::Prompt;
use crate::engines::{
    AgentEvent, EngineCommand, EngineOutput, Engines, HostToolCall, HostToolsConfig, LaunchConfig,
    PermissionAnswer, PermissionRequest, ThreadModels, static_efforts, static_models,
};
use crate::error::{Error, Result};
use crate::mcp::CormuxMcp;
use crate::model::{ThreadStatus, WorkspaceLifecycle};
use crate::store::Store;
use crate::transcript::Transcript;
use crate::workspaces::Workspaces;

/// Just over what a session gets to exit before it's killed.
const STOP_GRACE: Duration = Duration::from_secs(6);
/// A Claude CLI sitting idle this long gives its memory back; the next prompt resumes it.
const IDLE_REAP_AFTER: Duration = Duration::from_secs(30 * 60);
const IDLE_REAP_TICK: Duration = Duration::from_secs(5 * 60);

/// Told to the agent when its session starts: a thread starts off the default branch unless its
/// prompt says otherwise.
const BRANCH_INSTRUCTION: &str = "This thread starts on a new branch cut from the repo's default branch, with a worktree of its own. If the user's first message names another branch to start from, or asks to pick up earlier work, run `git reset --hard <branch>` (`origin/<branch>` if it only exists there) before anything else, while this branch has no commits of its own, and say which branch you started from. Ignore this for later messages, and don't do it for a branch the user only mentions.";

#[derive(Default)]
struct HoldState {
    paused: bool,
    queued: Vec<Prompt>,
}

#[derive(Clone)]
pub struct ThreadRuntime {
    engines: Arc<dyn Engines>,
    approvals: Arc<ApprovalBroker>,
    store: Store,
    transcript: Transcript,
    workspaces: Workspaces,
    host: CormuxMcp,
    approval_notify: broadcast::Sender<()>,
    /// Thread ids whose turn just ended (or whose engine exited).
    turn_end_notify: broadcast::Sender<String>,
    threads: Arc<Mutex<HashMap<String, ThreadSlot>>>,
    holds: Arc<Mutex<HashMap<String, HoldState>>>,
    /// One per thread, held from "no engine here" to the new one being registered, so two
    /// callers can't both launch a CLI for the same session (the second would replace the
    /// first's slot and shut it down mid-prompt).
    launch_locks: Arc<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
}

struct ThreadSlot {
    commands: mpsc::UnboundedSender<EngineCommand>,
    /// When anything last went to or came from the engine.
    last_active: Arc<Mutex<Instant>>,
    /// The CLI's process group, so quitting the app can take it down synchronously.
    pgid: Option<u32>,
}

impl ThreadSlot {
    fn touch(&self) {
        touch(&self.last_active);
    }
}

fn touch(last_active: &Mutex<Instant>) {
    if let Ok(mut last_active) = last_active.lock() {
        *last_active = Instant::now();
    }
}

/// What the runtime needs from the rest of the app.
pub struct RuntimeParts {
    pub engines: Arc<dyn Engines>,
    pub approvals: Arc<ApprovalBroker>,
    pub store: Store,
    pub transcript: Transcript,
    pub workspaces: Workspaces,
    pub host: CormuxMcp,
    pub approval_notify: broadcast::Sender<()>,
    pub turn_end_notify: broadcast::Sender<String>,
}

impl ThreadRuntime {
    pub fn new(parts: RuntimeParts) -> Self {
        Self {
            engines: parts.engines,
            approvals: parts.approvals,
            store: parts.store,
            transcript: parts.transcript,
            workspaces: parts.workspaces,
            host: parts.host,
            approval_notify: parts.approval_notify,
            turn_end_notify: parts.turn_end_notify,
            threads: Arc::new(Mutex::new(HashMap::new())),
            holds: Arc::new(Mutex::new(HashMap::new())),
            launch_locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Whether a live engine serves the thread. An engine whose pump has ended (the CLI
    /// exited or its output broke) is dropped here, so the caller launches a fresh one that
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

    /// Launches the thread's engine in its workspace unless one is running. Two prompts can race
    /// here (a user message and a child's report); only one launches.
    pub async fn ensure(&self, thread_id: &str, workspace_id: &str) -> Result<()> {
        if self.has_thread(thread_id) {
            return Ok(());
        }
        let lock = self
            .launch_locks
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?
            .entry(thread_id.to_string())
            .or_default()
            .clone();
        let _launching = lock.lock_owned().await;
        if self.has_thread(thread_id) {
            return Ok(());
        }
        self.start(thread_id, workspace_id, None).await
    }

    /// Launches a new engine for the thread in its workspace's worktree, replacing any it has,
    /// and sends it `prompt` first when there is one.
    pub async fn start(
        &self,
        thread_id: &str,
        workspace_id: &str,
        prompt: Option<Prompt>,
    ) -> Result<()> {
        let workspace = self.workspaces.get(workspace_id).await?;
        if workspace.status == WorkspaceLifecycle::TearingDown {
            return Err(Error::Workspace(format!(
                "workspace {workspace_id} is being torn down"
            )));
        }
        self.launch(thread_id, workspace.worktree()).await?;
        if let Some(prompt) =
            prompt.filter(|prompt| !prompt.text.is_empty() || !prompt.images.is_empty())
        {
            self.prompt(thread_id, prompt)?;
        }
        Ok(())
    }

    async fn launch(&self, thread_id: &str, cwd: PathBuf) -> Result<()> {
        let _ = self.stop(thread_id).await;
        let config = LaunchConfig {
            thread_id: thread_id.to_string(),
            cwd,
            model: self.store.thread_model(thread_id)?,
            effort: self.store.thread_effort(thread_id)?,
            resume: self.store.thread_session(thread_id)?,
            instructions: format!("{BRANCH_INSTRUCTION}\n\n{}", crate::media::instruction()),
            host_tools: Some(self.host_tools()),
        };
        let last_active = Arc::new(Mutex::new(Instant::now()));
        let (output, reported) = mpsc::unbounded_channel();
        let pump = Pump {
            thread_id: thread_id.to_string(),
            store: self.store.clone(),
            transcript: self.transcript.clone(),
            approvals: self.approvals.clone(),
            host: self.host.clone(),
            approval_notify: self.approval_notify.clone(),
            turn_end_notify: self.turn_end_notify.clone(),
            auto_approve_all: crate::approvals::run_everything_from_store(&self.store),
            outstanding: HashSet::new(),
            last_active: last_active.clone(),
        };
        tokio::spawn(pump.run(reported));
        let engine = self.engines.launch(config, output).await?;
        self.threads
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?
            .insert(
                thread_id.to_string(),
                ThreadSlot {
                    commands: engine.commands,
                    last_active,
                    pgid: engine.pgid,
                },
            );
        Ok(())
    }

    /// The Cormux tools every session gets. References' tool is withheld while that experiment
    /// is off.
    fn host_tools(&self) -> HostToolsConfig {
        let references = crate::mcp::references_enabled(&self.store);
        let mut instructions = crate::mcp::ORCHESTRATION_INSTRUCTION.to_string();
        if references {
            instructions.push_str("\n\n");
            instructions.push_str(crate::mcp::REFERENCES_INSTRUCTION);
        }
        HostToolsConfig {
            server_name: crate::mcp::SERVER_NAME.into(),
            allowed: crate::mcp::read_only_tools(),
            disallowed: if references {
                Vec::new()
            } else {
                vec![crate::mcp::ATTACH_ENTITY_TOOL.into()]
            },
            instructions,
        }
    }

    /// The thread's picked model and what it can switch to.
    pub fn thread_models(&self, thread_id: &str) -> Result<ThreadModels> {
        Ok(ThreadModels {
            current: self.store.thread_model(thread_id)?,
            options: static_models(),
        })
    }

    /// Saves the pick for future sessions and switches a live engine over now.
    pub fn set_thread_model(&self, thread_id: &str, model: Option<String>) -> Result<()> {
        self.store.set_thread_model(thread_id, model.as_deref())?;
        if self.has_thread(thread_id) {
            self.send(thread_id, EngineCommand::SetModel(model))?;
        }
        Ok(())
    }

    /// The thread's picked effort level and what it can switch to.
    pub fn thread_efforts(&self, thread_id: &str) -> Result<ThreadModels> {
        Ok(ThreadModels {
            current: self.store.thread_effort(thread_id)?,
            options: static_efforts(),
        })
    }

    /// Saves the pick for future sessions and switches a live engine over now.
    pub fn set_thread_effort(&self, thread_id: &str, effort: Option<String>) -> Result<()> {
        self.store.set_thread_effort(thread_id, effort.as_deref())?;
        if self.has_thread(thread_id) {
            self.send(thread_id, EngineCommand::SetEffort(effort))?;
        }
        Ok(())
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
        if let Err(error) = self.cancel(thread_id) {
            log::warn!("pausing {thread_id}: {error}");
        }
        let mut holds = self
            .holds
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?;
        holds.entry(thread_id.to_string()).or_default().paused = true;
        Ok(())
    }

    /// Ends a pause. The prompt is what queued during it, or `continue` if nothing did.
    pub fn release_thread(&self, thread_id: &str) -> Result<Prompt> {
        let mut holds = self
            .holds
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?;
        let slot = holds.entry(thread_id.to_string()).or_default();
        slot.paused = false;
        let queued = std::mem::take(&mut slot.queued);
        if queued.is_empty() {
            return Ok("continue".into());
        }
        Ok(Prompt::merge(queued))
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

    /// Signals every live engine's process group. For app quit, which can't wait on the
    /// pumps' graceful shutdown; without it the CLIs' shells and servers outlive the app.
    pub fn kill_all(&self) {
        let Ok(threads) = self.threads.lock() else {
            return;
        };
        for slot in threads.values() {
            let Some(pgid) = slot.pgid.and_then(|pid| libc::pid_t::try_from(pid).ok()) else {
                continue;
            };
            if pgid > 0 {
                // SAFETY: killpg takes plain integers and has no memory-safety preconditions.
                unsafe {
                    libc::killpg(pgid, libc::SIGTERM);
                }
            }
        }
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
            // The engine drops its receiver once it is down.
            let _ = tokio::time::timeout(STOP_GRACE, commands.closed()).await;
        }
        Ok(())
    }

    /// Shuts the engine down without waiting for it, and drops any paused turn and queued prompts.
    /// The next prompt launches a new engine.
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
            let runtime = self.clone();
            let id = id.clone();
            stops.spawn(async move {
                if let Err(error) = runtime.stop(&id).await {
                    log::warn!("stopping {id}'s engine: {error}");
                }
            });
        }
        stops.join_all().await;
        Ok(())
    }

    pub fn spawn_idle_reaper(&self) {
        let runtime = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(IDLE_REAP_TICK);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                interval.tick().await;
                runtime.reap_idle(IDLE_REAP_AFTER);
            }
        });
    }

    /// Shuts down engines nothing has touched for `idle_for`, returning their threads. Only one
    /// with a saved session goes, since that's what the next prompt resumes. A turn the engine is
    /// holding for background agents keeps its thread running, so that never reads as idle.
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
            .is_some_and(|thread| thread.has_status(ThreadStatus::Idle));
        let unanswered = self
            .store
            .pending_approvals_for_thread(thread_id)
            .map_or(true, |approvals| !approvals.is_empty());
        quiet && resumable && idle && !unanswered && !self.thread_is_held(thread_id)
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
}

/// One engine's way out to the rest of the app, in the order it reported. It keeps the approvals
/// the engine is waiting on, so a turn that ends (or an engine that dies) with one still open drops
/// it instead of leaving the dock badge counting a request nothing will ever answer.
struct Pump {
    thread_id: String,
    store: Store,
    transcript: Transcript,
    approvals: Arc<ApprovalBroker>,
    host: CormuxMcp,
    approval_notify: broadcast::Sender<()>,
    turn_end_notify: broadcast::Sender<String>,
    auto_approve_all: bool,
    outstanding: HashSet<String>,
    last_active: Arc<Mutex<Instant>>,
}

impl Pump {
    async fn run(mut self, mut reported: mpsc::UnboundedReceiver<EngineOutput>) {
        while let Some(output) = reported.recv().await {
            touch(&self.last_active);
            match output {
                EngineOutput::Event(event) => self.event(event),
                EngineOutput::Usage {
                    used_tokens,
                    context_size,
                    turn_cost_usd,
                } => self.usage(used_tokens, context_size, turn_cost_usd),
                EngineOutput::Permission(request) => self.permission(request),
                EngineOutput::HostTool(call) => self.host_tool(call),
                EngineOutput::Exited => {
                    self.transcript
                        .signal(&self.thread_id, AgentEvent::EngineExited { code: None });
                    self.ended();
                }
            }
        }
    }

    fn event(&mut self, event: AgentEvent) {
        if let AgentEvent::SessionStarted { session_id } = &event
            && let Err(error) = self.store.set_thread_session(&self.thread_id, session_id)
        {
            log::warn!("saving {}'s session: {error}", self.thread_id);
        }
        let turn_ended = matches!(event, AgentEvent::TurnEnd { .. });
        if turn_ended {
            self.settle();
        }
        if let Err(error) = self.transcript.append(&self.thread_id, event) {
            log::warn!("saving an event to {}: {error}", self.thread_id);
        }
        if turn_ended {
            self.ended();
        }
    }

    /// Idles the thread before anyone hears its turn ended, so nobody who does reads it as still
    /// running. A paused thread stays paused: pausing interrupts the CLI, which answers with a turn
    /// end of its own, and idling the thread then would strand its hold.
    fn settle(&self) {
        let paused = matches!(
            self.store.thread_by_id(&self.thread_id),
            Ok(Some(thread)) if thread.has_status(ThreadStatus::Paused)
        );
        if !paused
            && let Err(error) = self
                .store
                .set_thread_status(&self.thread_id, ThreadStatus::Idle)
        {
            log::warn!("idling {}: {error}", self.thread_id);
        }
    }

    /// The engine stopped waiting on anything: its turn is over, or it is gone.
    fn ended(&mut self) {
        self.cancel_outstanding();
        let _ = self.turn_end_notify.send(self.thread_id.clone());
    }

    /// Claude reports each turn's cost, so the thread's total adds them up across sessions.
    fn usage(&self, used_tokens: u64, context_size: u64, turn_cost_usd: Option<f64>) {
        let saved = match self.store.thread_by_id(&self.thread_id) {
            Ok(thread) => thread.and_then(|thread| thread.cost_usd),
            Err(error) => {
                log::warn!("reading {}'s cost: {error}", self.thread_id);
                None
            }
        };
        let cost_usd = match turn_cost_usd {
            Some(turn) => Some(saved.unwrap_or(0.0) + turn),
            None => saved,
        };
        let usage = AgentEvent::Usage {
            used_tokens,
            context_size,
            cost_usd,
        };
        if let Err(error) = self.transcript.append(&self.thread_id, usage) {
            log::warn!("saving {}'s usage: {error}", self.thread_id);
        }
        if let Err(error) =
            self.store
                .set_thread_usage(&self.thread_id, used_tokens, context_size, cost_usd)
        {
            log::warn!("saving {}'s usage: {error}", self.thread_id);
        }
    }

    fn permission(&mut self, request: PermissionRequest) {
        // Only the user can answer a question, so no auto-approve setting applies to one.
        if request.questions.is_empty() && self.auto_approve_all {
            self.transcript.signal(&self.thread_id, request.event(true));
            let _ = request.respond.send(PermissionAnswer::Allow);
            return;
        }
        // Registered before the UI hears of it, so an answer can't arrive for a request the
        // broker doesn't know yet.
        let decided = self.approvals.register(request.id.clone());
        self.outstanding.insert(request.id.clone());
        let event = request.event(false);
        let recorded = if request.questions.is_empty() {
            crate::approvals::record_pending_permission(
                &self.store,
                &self.transcript,
                &self.thread_id,
                &event,
            )
        } else {
            crate::approvals::record_pending_question(
                &self.store,
                &self.transcript,
                &self.thread_id,
                &event,
                request.questions,
            )
        };
        match recorded {
            Ok(()) => {
                let _ = self.approval_notify.send(());
            }
            Err(error) => log::warn!("recording {}'s approval: {error}", self.thread_id),
        }
        self.transcript.signal(
            &self.thread_id,
            AgentEvent::CurrentTool {
                id: request.tool_call_id,
                title: request.tool,
            },
        );
        let respond = request.respond;
        tokio::spawn(async move {
            // A closed channel means the request was cancelled: the turn it belonged to is over.
            if let Ok(decision) = decided.await {
                let _ = respond.send(answer(decision));
            }
        });
    }

    fn host_tool(&self, call: HostToolCall) {
        let host = self.host.clone();
        let thread_id = self.thread_id.clone();
        tokio::spawn(async move {
            let reply = if call.server == crate::mcp::SERVER_NAME {
                host.handle_message(&thread_id, &call.message).await
            } else {
                crate::mcp::unknown_server_reply(&call.message, &call.server)
            };
            let _ = call.respond.send(reply);
        });
    }

    /// The engine stops waiting for an answer once its turn ends, so neither should the broker,
    /// nor the store: an engine shut down mid-question (new session, stop, replaced) sends no
    /// turn end, so without this its row would stay `pending` and badge the thread forever.
    fn cancel_outstanding(&mut self) {
        for id in self.outstanding.drain() {
            self.approvals.cancel(&id);
            if let Err(error) = self.store.deny_pending_approval(&id) {
                log::warn!("denying abandoned approval {id}: {error}");
            }
        }
    }
}

impl Drop for Pump {
    fn drop(&mut self) {
        self.cancel_outstanding();
    }
}

fn answer(decision: ApprovalDecision) -> PermissionAnswer {
    match decision {
        ApprovalDecision::Approved => PermissionAnswer::Allow,
        ApprovalDecision::Denied { message } => PermissionAnswer::Deny { message },
        ApprovalDecision::Answered { answers } => PermissionAnswer::Answer { answers },
    }
}

#[cfg(test)]
mod tests;
