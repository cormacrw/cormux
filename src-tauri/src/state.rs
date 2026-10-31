use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::AppHandle;
use tauri_specta::Event;
use tokio::sync::{RwLock, broadcast};

use crate::app::WorkspaceAppService;
use crate::clickup::ClickupClient;
use crate::git::{FetchScheduler, LiveDiffEngine};
use crate::ipc::events::StateChanged;
use crate::ipc::subscriptions::Subscriptions;
use crate::ipc::types::StateChangeKind;
use crate::orchestration::ReportQueue;
use crate::{
    approvals::ApprovalBroker, engines::EngineRegistry, git::Git, github::PrSyncScheduler,
    llm::LlmClient, mcp::CormuxMcp, metrics::Metrics, process::ProcessSupervisor,
    shell_env::ShellEnv, store::Store, workspace::WorkspaceManager,
};

/// Changes closer together than this reach the webview as one `StateChanged`. The webview
/// answers every one with a full snapshot fetch, so a burst (an approval resolving, its
/// counts, its toast, the turn ending) costs one fetch instead of one each. The first change
/// in a quiet spell goes out at once, so nothing waits on the window.
const CHANGE_WINDOW: Duration = Duration::from_millis(50);

/// Process-wide core state. The webview never holds this; IPC commands borrow it.
pub struct AppState {
    pub event_version: AtomicU64,
    changes: Mutex<ChangeCoalescer>,
    pub shell_env: Arc<RwLock<ShellEnv>>,
    pub git: Git,
    pub workspace: WorkspaceManager,
    pub engines: EngineRegistry,
    pub approvals: Arc<ApprovalBroker>,
    pub mcp: CormuxMcp,
    pub process: ProcessSupervisor,
    pub apps: WorkspaceAppService,
    pub clickup: ClickupClient,
    pub llm: LlmClient,
    pub store: Store,
    pub metrics: Metrics,
    pub diffs: LiveDiffEngine,
    pub fetch: FetchScheduler,
    pub pr_sync: PrSyncScheduler,
    pub approval_notify: broadcast::Sender<()>,
    pub turn_end_notify: broadcast::Sender<String>,
    pub subscriptions: Subscriptions,
    pub reports: ReportQueue,
}

impl AppState {
    pub fn new() -> Self {
        let shell_env = Arc::new(RwLock::new(ShellEnv::new()));
        let git = Git::new(shell_env.clone());
        let diffs = LiveDiffEngine::new(git.clone());
        let fetch = FetchScheduler::new(git.clone());
        let pr_sync = PrSyncScheduler::new(git.clone(), shell_env.clone());
        let process = ProcessSupervisor::new(shell_env.clone());
        let apps = WorkspaceAppService::new();
        let workspace = WorkspaceManager::new(git.clone(), diffs.clone(), fetch.clone());
        let store = Store::new();
        let approvals = Arc::new(ApprovalBroker::new());
        let (approval_notify, _) = broadcast::channel(32);
        let (turn_end_notify, _) = broadcast::channel(64);
        let mcp = CormuxMcp::new(
            store.clone(),
            process.clone(),
            approvals.clone(),
            apps.clone(),
        );
        let engines = EngineRegistry::new(
            shell_env.clone(),
            approvals.clone(),
            store.clone(),
            mcp.clone(),
            approval_notify.clone(),
            turn_end_notify.clone(),
        );
        Self {
            event_version: AtomicU64::new(0),
            changes: Mutex::new(ChangeCoalescer::default()),
            shell_env: shell_env.clone(),
            git,
            workspace,
            engines,
            approvals,
            mcp,
            process,
            apps,
            clickup: ClickupClient::new(),
            llm: LlmClient::new(shell_env),
            store,
            metrics: Metrics::new(),
            diffs,
            fetch,
            pr_sync,
            approval_notify,
            turn_end_notify,
            subscriptions: Subscriptions::new(),
            reports: ReportQueue::default(),
        }
    }

    pub fn snapshot_version(&self) -> u64 {
        self.event_version.load(Ordering::SeqCst)
    }

    pub fn bump_event_version(&self) -> u64 {
        self.event_version.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Tells the webview something it mirrors has changed. Emits a `StateChanged`, merging
    /// changes that land within `CHANGE_WINDOW` of the last one into a single trailing event.
    /// Prefer this to emitting `StateChanged` by hand.
    ///
    /// The version only moves when an event goes out, so consecutive events are numbered
    /// consecutively: the webview treats a jump as a dropped event and does a full resync,
    /// which a merged burst must not look like.
    pub fn notify(&self, app: &AppHandle, kind: StateChangeKind) {
        let decision = match self.changes.lock() {
            Ok(mut changes) => changes.record(kind, Instant::now()),
            Err(_) => Decision::EmitNow(kind),
        };
        match decision {
            Decision::EmitNow(kind) => {
                let version = self.bump_event_version();
                let _ = StateChanged { version, kind }.emit(app);
            }
            Decision::Schedule(delay) => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(delay).await;
                    let state = tauri::Manager::state::<AppState>(&app);
                    let kind = match state.changes.lock() {
                        Ok(mut changes) => changes.flush(Instant::now()),
                        Err(_) => Some(StateChangeKind::WorkspaceStatus),
                    };
                    if let Some(kind) = kind {
                        let version = state.bump_event_version();
                        let _ = StateChanged { version, kind }.emit(&app);
                    }
                });
            }
            Decision::Merged => {}
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// What `notify` should do with a change, given the ones before it.
#[derive(Debug, PartialEq, Eq)]
enum Decision {
    EmitNow(StateChangeKind),
    /// Wait this long, then `flush`.
    Schedule(Duration),
    /// A flush is already scheduled and will carry this change.
    Merged,
}

/// Folds a burst of changes into one event. Kinds merge by what the webview does with them:
/// `BehindCounts` also refreshes the repo rows, so it wins over the plain refetch kinds.
#[derive(Debug, Default)]
struct ChangeCoalescer {
    last_emit: Option<Instant>,
    pending: Option<StateChangeKind>,
}

impl ChangeCoalescer {
    fn record(&mut self, kind: StateChangeKind, now: Instant) -> Decision {
        if let Some(pending) = self.pending.take() {
            self.pending = Some(merge_kinds(pending, kind));
            return Decision::Merged;
        }
        match self.last_emit {
            Some(last) if now.duration_since(last) < CHANGE_WINDOW => {
                self.pending = Some(kind);
                Decision::Schedule(CHANGE_WINDOW - now.duration_since(last))
            }
            _ => {
                self.last_emit = Some(now);
                Decision::EmitNow(kind)
            }
        }
    }

    fn flush(&mut self, now: Instant) -> Option<StateChangeKind> {
        let kind = self.pending.take()?;
        self.last_emit = Some(now);
        Some(kind)
    }
}

/// The kind that makes the webview do the most, so a merged event never does less than each
/// of the changes in it would have. `BehindCounts` also refreshes the repo rows; a workspace
/// or approval change may have written a step into a thread, which the webview re-reads
/// for; the rest only need the snapshot (see `KINDS_WITHOUT_THREAD_EVENTS` in the webview).
fn merge_kinds(a: StateChangeKind, b: StateChangeKind) -> StateChangeKind {
    fn weight(kind: StateChangeKind) -> u8 {
        match kind {
            StateChangeKind::BehindCounts => 3,
            StateChangeKind::WorkspaceStatus | StateChangeKind::ApprovalCounts => 2,
            StateChangeKind::Toast => 0,
            StateChangeKind::PrSync
            | StateChangeKind::Environment
            | StateChangeKind::WorkspaceApp => 1,
        }
    }
    if weight(b) > weight(a) { b } else { a }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_without_async_runtime() {
        let _state = AppState::new();
    }

    #[test]
    fn a_lone_change_goes_out_at_once_and_a_burst_becomes_one_trailing_event() {
        let mut changes = ChangeCoalescer::default();
        let start = Instant::now();
        assert_eq!(
            changes.record(StateChangeKind::WorkspaceStatus, start),
            Decision::EmitNow(StateChangeKind::WorkspaceStatus)
        );
        let later = start + Duration::from_millis(10);
        assert_eq!(
            changes.record(StateChangeKind::ApprovalCounts, later),
            Decision::Schedule(CHANGE_WINDOW - Duration::from_millis(10))
        );
        assert_eq!(
            changes.record(StateChangeKind::BehindCounts, later),
            Decision::Merged
        );
        assert_eq!(
            changes.record(StateChangeKind::PrSync, later),
            Decision::Merged
        );
        let flushed = start + CHANGE_WINDOW;
        assert_eq!(
            changes.flush(flushed),
            Some(StateChangeKind::BehindCounts),
            "the kind the webview does the most with wins"
        );
        assert_eq!(changes.flush(flushed), None);
        // The flush counts as the last emit, so the next change waits again.
        assert!(matches!(
            changes.record(StateChangeKind::WorkspaceStatus, flushed),
            Decision::Schedule(_)
        ));
        let mut mixed = ChangeCoalescer::default();
        mixed.record(StateChangeKind::PrSync, start);
        mixed.record(StateChangeKind::WorkspaceStatus, later);
        mixed.record(StateChangeKind::WorkspaceApp, later);
        assert_eq!(
            mixed.flush(flushed),
            Some(StateChangeKind::WorkspaceStatus),
            "a change that may have written thread steps isn't hidden by a later lighter one"
        );
        let mut quiet = ChangeCoalescer::default();
        quiet.record(StateChangeKind::WorkspaceStatus, start);
        assert_eq!(
            quiet.record(StateChangeKind::PrSync, start + CHANGE_WINDOW),
            Decision::EmitNow(StateChangeKind::PrSync)
        );
    }
}
