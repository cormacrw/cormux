//! Each thread's live engine: launching one in the thread's worktree when a prompt needs it,
//! routing what it reports (events into the transcript, permission requests to approvals, tool
//! calls to whoever serves the host's tools), adding up its cost, reporting each turn's end once
//! with the engine it came from, and shutting down engines that sit idle. An engine shut down on
//! purpose (stopped, discarded, reaped or replaced) reports no end: whoever shut it down has
//! already settled its thread.

#[cfg(test)]
pub mod fake;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, mpsc, oneshot};

use crate::approvals::{ApprovalBroker, ApprovalDecision, ApprovalPolicy};
use crate::engines::prompt::Prompt;
use crate::engines::{
    AgentEvent, EngineCommand, EngineOutput, Engines, HostToolCall, HostToolsConfig, LaunchConfig,
    PermissionAnswer, PermissionRequest, ThreadModels, static_efforts, static_models,
};
use crate::error::{Error, Result};
use crate::model::{ThreadStatus, WorkspaceLifecycle};
use crate::settings::Settings;
use crate::store::Store;
use crate::store::types::ThreadRow;
use crate::transcript::{HostEvent, Transcript, TranscriptEvent};
use crate::workspaces::{WorkspaceRecord, Workspaces};

/// A Claude CLI sitting idle this long gives its memory back; the next prompt resumes it.
const IDLE_REAP_AFTER: Duration = Duration::from_secs(30 * 60);
const IDLE_REAP_TICK: Duration = Duration::from_secs(5 * 60);

/// A message for the host's tools from a thread's engine, waiting for its reply.
#[derive(Debug)]
pub struct HostRequest {
    /// The thread whose session sent it, so no tool argument can claim to be another.
    pub thread_id: String,
    pub call: HostToolCall,
}

/// The host's tools as a session is told them, for the settings it starts with and its thread.
pub type HostToolsFor = Arc<dyn Fn(&Settings, &ThreadRow) -> HostToolsConfig + Send + Sync>;

/// What a session is told about the workspace it runs in, for the thread it serves and whether it
/// resumes a saved session.
pub type BriefFor = Arc<dyn Fn(&WorkspaceRecord, &str, bool) -> String + Send + Sync>;

/// A turn's end, or its engine's, as the engine that served the thread reported it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnEnd {
    pub thread_id: String,
    /// The launch it came from; see [`ThreadRuntime::is_current`].
    pub engine: u64,
    /// The turn was cancelled (paused or stopped) before it ended: the thread was already moved
    /// by whoever cancelled it.
    pub interrupted: bool,
    /// The engine already has a message it starts its next turn on.
    pub queued: bool,
}

/// Where an engine is in its turn, shared by its slot (which sends it prompts and cancels) and
/// its pump (which sees the turn end).
#[derive(Debug, Default)]
struct EngineTurn {
    /// Prompted, or reporting, since its last turn end.
    open: bool,
    /// Cancelled while open; the turn's end says so.
    interrupted: bool,
}

#[derive(Clone)]
pub struct ThreadRuntime {
    engines: Arc<dyn Engines>,
    approvals: Arc<ApprovalBroker>,
    store: Store,
    transcript: Transcript,
    workspaces: Workspaces,
    host_tools: HostToolsFor,
    brief: BriefFor,
    host_requests: mpsc::UnboundedSender<HostRequest>,
    /// Taken once, by whoever serves the host's tools; requests wait in it until then.
    host_inbox: Arc<Mutex<Option<mpsc::UnboundedReceiver<HostRequest>>>>,
    ended: broadcast::Sender<TurnEnd>,
    threads: Arc<Mutex<HashMap<String, ThreadSlot>>>,
    /// Each thread's newest engine that nobody shut down, by launch number. Kept apart from
    /// `threads`, which forgets an engine once it has exited, so an engine that died on its own
    /// still counts as current until the next launch.
    current: Arc<Mutex<HashMap<String, u64>>>,
    launches: Arc<AtomicU64>,
    /// One per thread; see [`ThreadOp`].
    operations: Arc<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
}

/// A hold on one thread's engine. A thread's operations run one at a time, and only one holding
/// this launches, prompts, interrupts or shuts down its engine: two callers can't both launch a
/// CLI for the same session (the second would replace the first's and shut it down mid-prompt),
/// and nothing stops an engine still launching. `turn` takes it for each of its operations.
pub struct ThreadOp {
    thread_id: String,
    _held: tokio::sync::OwnedMutexGuard<()>,
}

impl ThreadOp {
    pub fn thread_id(&self) -> &str {
        &self.thread_id
    }
}

struct ThreadSlot {
    commands: mpsc::UnboundedSender<EngineCommand>,
    turn: Arc<Mutex<EngineTurn>>,
    /// When anything last went to or came from the engine.
    last_active: Arc<Mutex<Instant>>,
    /// The CLI's process group, so quitting the app can take it down synchronously.
    pgid: Option<u32>,
    /// How long `stop` waits for the engine to go down, as its adapter says.
    stops_within: Duration,
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

pub struct RuntimeParts {
    pub engines: Arc<dyn Engines>,
    pub approvals: Arc<ApprovalBroker>,
    pub store: Store,
    pub transcript: Transcript,
    pub workspaces: Workspaces,
    pub host_tools: HostToolsFor,
    pub brief: BriefFor,
}

impl ThreadRuntime {
    pub fn new(parts: RuntimeParts) -> Self {
        let (host_requests, host_inbox) = mpsc::unbounded_channel();
        Self {
            engines: parts.engines,
            approvals: parts.approvals,
            store: parts.store,
            transcript: parts.transcript,
            workspaces: parts.workspaces,
            host_tools: parts.host_tools,
            brief: parts.brief,
            host_requests,
            host_inbox: Arc::new(Mutex::new(Some(host_inbox))),
            ended: broadcast::channel(64).0,
            threads: Arc::new(Mutex::new(HashMap::new())),
            current: Arc::new(Mutex::new(HashMap::new())),
            launches: Arc::new(AtomicU64::new(0)),
            operations: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Turns that just ended, or engines that exited mid-turn, once each. The open approvals of
    /// that engine are already cancelled; the thread's status is the listener's.
    pub fn turn_ends(&self) -> broadcast::Receiver<TurnEnd> {
        self.ended.subscribe()
    }

    /// Whether `engine` is the thread's newest launch and nobody has shut it down since. An end
    /// from any other engine is stale: the thread has moved on.
    pub fn is_current(&self, thread_id: &str, engine: u64) -> bool {
        is_current(&self.current, thread_id, engine)
    }

    /// The thread's current engine, made up if it never had one, for tests that report its
    /// turn's end themselves.
    #[cfg(test)]
    pub fn current_engine(&self, thread_id: &str) -> u64 {
        let mut current = self.current.lock().unwrap();
        *current
            .entry(thread_id.to_string())
            .or_insert_with(|| self.launches.fetch_add(1, Ordering::Relaxed) + 1)
    }

    /// The engines' messages for the host's tools. There is one server; it takes them once.
    pub fn take_host_requests(&self) -> Option<mpsc::UnboundedReceiver<HostRequest>> {
        self.host_inbox.lock().ok()?.take()
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

    /// Waits for the thread's other operations to finish; see [`ThreadOp`].
    pub async fn operation(&self, thread_id: &str) -> ThreadOp {
        let held = self.operation_lock(thread_id).lock_owned().await;
        ThreadOp {
            thread_id: thread_id.to_string(),
            _held: held,
        }
    }

    fn operation_lock(&self, thread_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.operations
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .entry(thread_id.to_string())
            .or_default()
            .clone()
    }

    /// Launches the thread's engine in its workspace unless one is running.
    pub async fn ensure(&self, op: &ThreadOp, workspace_id: &str) -> Result<()> {
        if self.has_thread(op.thread_id()) {
            return Ok(());
        }
        self.launch_in(op.thread_id(), workspace_id).await
    }

    /// Launches a new engine for the thread in its workspace's worktree, replacing any it has.
    pub async fn start(&self, op: &ThreadOp, workspace_id: &str) -> Result<()> {
        self.launch_in(op.thread_id(), workspace_id).await
    }

    /// A workspace being torn down launches nothing: teardown stops its engines once any launch
    /// already going has registered.
    async fn launch_in(&self, thread_id: &str, workspace_id: &str) -> Result<()> {
        let workspace = self.workspaces.get(workspace_id).await?;
        if workspace.status == WorkspaceLifecycle::TearingDown {
            return Err(Error::Workspace(format!(
                "workspace {workspace_id} is being torn down"
            )));
        }
        self.launch(thread_id, &workspace).await
    }

    async fn launch(&self, thread_id: &str, workspace: &WorkspaceRecord) -> Result<()> {
        let _ = self.shut_down(thread_id).await;
        let settings = Settings::load(&self.store);
        let thread = self
            .store
            .thread_by_id(thread_id)?
            .ok_or_else(|| Error::Engine(format!("unknown thread {thread_id}")))?;
        let resume = self.store.thread_session(thread_id)?;
        let config = LaunchConfig {
            thread_id: thread_id.to_string(),
            cwd: workspace.worktree(),
            model: self.store.thread_model(thread_id)?,
            effort: self.store.thread_effort(thread_id)?,
            instructions: (self.brief)(workspace, thread_id, resume.is_some()),
            resume,
            host_tools: Some((self.host_tools)(&settings, &thread)),
        };
        let last_active = Arc::new(Mutex::new(Instant::now()));
        let turn = Arc::new(Mutex::new(EngineTurn::default()));
        let engine = self.launches.fetch_add(1, Ordering::Relaxed) + 1;
        let (output, reported) = mpsc::unbounded_channel();
        let (launched, launch_settled) = oneshot::channel();
        let pump = Pump {
            thread_id: thread_id.to_string(),
            engine,
            current: self.current.clone(),
            turn: turn.clone(),
            store: self.store.clone(),
            transcript: self.transcript.clone(),
            approvals: self.approvals.clone(),
            host_requests: self.host_requests.clone(),
            ended: self.ended.clone(),
            policy: ApprovalPolicy::from(&settings),
            last_active: last_active.clone(),
        };
        // Running before the launch returns: the engine's handshake can already call the host's
        // tools. A launch that fails drops `launched`, so the pump reports no end for it.
        tokio::spawn(pump.run(reported, launch_settled));
        let handle = self.engines.launch(config, output).await?;
        self.threads
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?
            .insert(
                thread_id.to_string(),
                ThreadSlot {
                    commands: handle.commands,
                    turn,
                    last_active,
                    pgid: handle.pgid,
                    stops_within: handle.stops_within,
                },
            );
        self.current
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?
            .insert(thread_id.to_string(), engine);
        let _ = launched.send(());
        Ok(())
    }

    pub fn thread_models(&self, thread_id: &str) -> Result<ThreadModels> {
        Ok(ThreadModels {
            current: self.store.thread_model(thread_id)?,
            options: static_models(),
        })
    }

    pub fn set_thread_model(&self, thread_id: &str, model: Option<String>) -> Result<()> {
        self.store.set_thread_model(thread_id, model.as_deref())?;
        if self.has_thread(thread_id) {
            self.send(thread_id, EngineCommand::SetModel(model))?;
        }
        Ok(())
    }

    pub fn thread_efforts(&self, thread_id: &str) -> Result<ThreadModels> {
        Ok(ThreadModels {
            current: self.store.thread_effort(thread_id)?,
            options: static_efforts(),
        })
    }

    pub fn set_thread_effort(&self, thread_id: &str, effort: Option<String>) -> Result<()> {
        self.store.set_thread_effort(thread_id, effort.as_deref())?;
        if self.has_thread(thread_id) {
            self.send(thread_id, EngineCommand::SetEffort(effort))?;
        }
        Ok(())
    }

    pub fn prompt(&self, op: &ThreadOp, prompt: impl Into<Prompt>) -> Result<()> {
        self.send_with(
            op.thread_id(),
            EngineCommand::Prompt(prompt.into()),
            |turn| {
                turn.open = true;
            },
        )
    }

    /// Whether the thread's engine has been prompted, or reported, since its last turn end.
    pub fn in_turn(&self, thread_id: &str) -> bool {
        self.threads.lock().is_ok_and(|threads| {
            threads
                .get(thread_id)
                .is_some_and(|slot| slot.turn.lock().is_ok_and(|turn| turn.open))
        })
    }

    /// Closes the engine's turn as its pump does on a turn end, for tests that report the end
    /// themselves.
    #[cfg(test)]
    pub fn close_turn(&self, thread_id: &str) {
        if let Some(slot) = self.threads.lock().unwrap().get(thread_id) {
            slot.turn.lock().unwrap().open = false;
        }
    }

    /// Interrupts the engine's turn. If one is open, its end will say it was interrupted.
    pub fn cancel(&self, op: &ThreadOp) -> Result<()> {
        self.send_with(op.thread_id(), EngineCommand::Cancel, |turn| {
            turn.interrupted |= turn.open;
        })
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

    /// Shuts the engine down and waits, as long as its adapter says it takes, for it to go.
    pub async fn stop(&self, op: &ThreadOp) -> Result<()> {
        self.shut_down(op.thread_id()).await
    }

    async fn shut_down(&self, thread_id: &str) -> Result<()> {
        let commands = {
            let mut threads = self
                .threads
                .lock()
                .map_err(|error| Error::Engine(error.to_string()))?;
            self.dismiss(thread_id);
            threads
                .remove(thread_id)
                .map(|slot| (slot.commands, slot.stops_within))
        };
        if let Some((commands, stops_within)) = commands {
            let _ = commands.send(EngineCommand::Shutdown);
            let _ = tokio::time::timeout(stops_within, commands.closed()).await;
        }
        Ok(())
    }

    /// Shuts the engine down without waiting for it. The next prompt launches a new engine.
    pub fn discard(&self, op: &ThreadOp) -> Result<()> {
        let thread_id = op.thread_id();
        let slot = {
            let mut threads = self
                .threads
                .lock()
                .map_err(|error| Error::Engine(error.to_string()))?;
            self.dismiss(thread_id);
            threads.remove(thread_id)
        };
        if let Some(slot) = slot {
            let _ = slot.commands.send(EngineCommand::Shutdown);
        }
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
    /// holding for background agents keeps its thread running, and a paused one is paused, so
    /// neither reads as idle. A thread in the middle of an operation waits for the next tick.
    fn reap_idle(&self, idle_for: Duration) -> Vec<String> {
        let Ok(mut threads) = self.threads.lock() else {
            return Vec::new();
        };
        let idle: Vec<String> = threads
            .iter()
            .filter(|(thread_id, slot)| self.can_reap(thread_id, slot, idle_for))
            .map(|(thread_id, _)| thread_id.clone())
            .collect();
        let mut reaped = Vec::new();
        for thread_id in idle {
            let lock = self.operation_lock(&thread_id);
            let Ok(_held) = lock.try_lock() else {
                continue;
            };
            self.dismiss(&thread_id);
            if let Some(slot) = threads.remove(&thread_id) {
                let _ = slot.commands.send(EngineCommand::Shutdown);
            }
            reaped.push(thread_id);
        }
        reaped
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
            .is_some_and(|thread| thread.status == ThreadStatus::Idle);
        let unanswered = self
            .store
            .pending_approvals_for_thread(thread_id)
            .map_or(true, |approvals| !approvals.is_empty());
        quiet && resumable && idle && !unanswered
    }

    fn send(&self, thread_id: &str, command: EngineCommand) -> Result<()> {
        self.send_with(thread_id, command, |_| {})
    }

    /// Sends `command`, moving the engine's turn by `turn` once it's on its way.
    fn send_with(
        &self,
        thread_id: &str,
        command: EngineCommand,
        turn: impl FnOnce(&mut EngineTurn),
    ) -> Result<()> {
        let threads = self
            .threads
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?;
        let slot = threads
            .get(thread_id)
            .ok_or_else(|| Error::Engine(format!("no engine for thread {thread_id}")))?;
        slot.touch();
        let mut state = slot
            .turn
            .lock()
            .map_err(|error| Error::Engine(error.to_string()))?;
        slot.commands
            .send(command)
            .map_err(|_| Error::Engine("engine process is gone".into()))?;
        turn(&mut state);
        Ok(())
    }

    /// The thread's engine is being shut down on purpose, so nothing it reports ends a turn.
    fn dismiss(&self, thread_id: &str) {
        if let Ok(mut current) = self.current.lock() {
            current.remove(thread_id);
        }
    }
}

fn is_current(current: &Mutex<HashMap<String, u64>>, thread_id: &str, engine: u64) -> bool {
    current
        .lock()
        .is_ok_and(|current| current.get(thread_id) == Some(&engine))
}

/// One engine's way out to the rest of the app, in the order it reported.
struct Pump {
    thread_id: String,
    engine: u64,
    current: Arc<Mutex<HashMap<String, u64>>>,
    turn: Arc<Mutex<EngineTurn>>,
    store: Store,
    transcript: Transcript,
    approvals: Arc<ApprovalBroker>,
    host_requests: mpsc::UnboundedSender<HostRequest>,
    ended: broadcast::Sender<TurnEnd>,
    policy: ApprovalPolicy,
    last_active: Arc<Mutex<Instant>>,
}

impl Pump {
    async fn run(
        self,
        mut reported: mpsc::UnboundedReceiver<EngineOutput>,
        launch_settled: oneshot::Receiver<()>,
    ) {
        let mut launch = LaunchOutcome::Pending(launch_settled);
        let mut exited = false;
        while let Some(output) = reported.recv().await {
            touch(&self.last_active);
            match output {
                EngineOutput::Event(event) => self.event(event),
                EngineOutput::TurnEnded {
                    stop_reason,
                    error,
                    next_queued,
                } => self.turn_ended(HostEvent::TurnEnd { stop_reason, error }, next_queued),
                EngineOutput::Usage {
                    used_tokens,
                    context_size,
                    turn_cost_usd,
                } => self.usage(used_tokens, context_size, turn_cost_usd),
                EngineOutput::Permission(request) => {
                    self.opened();
                    self.permission(request)
                }
                EngineOutput::HostTool(call) => self.host_tool(call),
                EngineOutput::Exited => {
                    exited = true;
                    if launch.succeeded().await {
                        self.transcript
                            .signal(&self.thread_id, HostEvent::EngineExited);
                        self.gone();
                    }
                }
            }
        }
        // An engine whose output broke off without saying it exited waits on nothing either.
        if !exited && launch.succeeded().await {
            self.gone();
        }
    }

    fn event(&self, event: AgentEvent) {
        if let AgentEvent::SessionStarted { session_id } = &event
            && let Err(error) = self.store.set_thread_session(&self.thread_id, session_id)
        {
            log::warn!("saving {}'s session: {error}", self.thread_id);
        }
        self.opened();
        self.save(event);
    }

    /// Saved, so after a reload the thread still says how its turn ended.
    fn turn_ended(&self, end: HostEvent, next_queued: bool) {
        self.save(end);
        self.ended(next_queued);
    }

    fn save(&self, event: impl Into<TranscriptEvent>) {
        if let Err(error) = self.transcript.append(&self.thread_id, event) {
            log::warn!("saving an event to {}: {error}", self.thread_id);
        }
    }

    /// Reporting means the engine is in a turn, whether or not it was prompted for it (a
    /// message queued behind the last turn starts the next by itself).
    fn opened(&self) {
        if let Ok(mut turn) = self.turn.lock() {
            turn.open = true;
        }
    }

    /// The engine is gone. That ends its turn only if one was open: a turn that already
    /// reported its end (as a failed read does just before the output closes) ends once.
    fn gone(&self) {
        let open = self.turn.lock().is_ok_and(|turn| turn.open);
        if open {
            self.ended(false);
        } else {
            self.approvals.cancel_for_engine(self.engine);
        }
    }

    /// The engine stopped waiting on anything: its turn is over, or it is gone. Its open
    /// requests go either way; the end is reported only while it is the thread's current engine.
    fn ended(&self, queued: bool) {
        let interrupted = match self.turn.lock() {
            Ok(mut turn) => {
                turn.open = false;
                std::mem::take(&mut turn.interrupted)
            }
            Err(_) => false,
        };
        self.approvals.cancel_for_engine(self.engine);
        if is_current(&self.current, &self.thread_id, self.engine) {
            let _ = self.ended.send(TurnEnd {
                thread_id: self.thread_id.clone(),
                engine: self.engine,
                interrupted,
                queued,
            });
        }
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
        let usage = HostEvent::Usage {
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

    fn permission(&self, request: PermissionRequest) {
        let decided = self
            .approvals
            .request(&self.thread_id, self.engine, &request, self.policy);
        let respond = request.respond;
        tokio::spawn(async move {
            // A closed channel means the request was cancelled: the turn it belonged to is over.
            if let Ok(decision) = decided.await {
                let _ = respond.send(answer(decision));
            }
        });
    }

    fn host_tool(&self, call: HostToolCall) {
        // With no server left the call is dropped, and the engine answers its tool with an error.
        let _ = self.host_requests.send(HostRequest {
            thread_id: self.thread_id.clone(),
            call,
        });
    }
}

/// Whether the engine a pump serves came up. One that never did had no turn to end: the launch's
/// caller gets its error instead.
enum LaunchOutcome {
    Pending(oneshot::Receiver<()>),
    Settled(bool),
}

impl LaunchOutcome {
    async fn succeeded(&mut self) -> bool {
        if let Self::Pending(settled) = self {
            *self = Self::Settled(settled.await.is_ok());
        }
        matches!(self, Self::Settled(true))
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
