//! How the core tells the user something happened: state changes the webview mirrors, toasts
//! and the dock badge. [`Notifier`] is the only way out for these; `ipc::events::TauriOutput`
//! carries them to the webview and the dock, and tests use `RecordingOutput`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use specta::Type;

/// Changes closer together than this reach the webview as one `StateChanged`. The webview
/// answers every one with a full snapshot fetch, so a burst (an approval resolving, its
/// counts, the turn ending) costs one fetch instead of one each. The first change in a quiet
/// spell goes out at once, so nothing waits on the window.
const CHANGE_WINDOW: Duration = Duration::from_millis(50);

/// What the webview has to re-read for a change, besides its snapshot, which it always
/// refetches.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    /// `origin` moved, so the repo rows' behind and ahead counts are stale.
    pub repo_git: bool,
}

impl Change {
    fn merge(self, other: Self) -> Self {
        Self {
            repo_git: self.repo_git || other.repo_git,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ToastTone {
    Ok,
    Bad,
    Default,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ToastPart {
    Text { value: String },
    Code { value: String },
}

impl ToastPart {
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text {
            value: value.into(),
        }
    }

    pub fn code(value: impl Into<String>) -> Self {
        Self::Code {
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ToastRaisedPayload {
    pub tone: ToastTone,
    pub parts: Vec<ToastPart>,
    pub workspace_id: Option<String>,
    /// With `workspace_id`, clicking the toast opens this thread's tab.
    pub thread_id: Option<String>,
}

pub trait UserOutput: Send + Sync {
    fn state_changed(&self, version: u64, change: Change);
    fn toast(&self, payload: ToastRaisedPayload);
    fn dock_badge(&self, count: Option<i64>);
}

/// Refreshes the webview, and with it the dock badge, whenever an agent asks for an approval.
pub fn start(bus: &crate::bus::Bus, out: &Notifier) {
    let out = out.clone();
    bus.listen(move |event| {
        if let crate::bus::DomainEvent::ApprovalRequested { .. } = event {
            out.changed();
        }
        std::future::ready(())
    });
}

/// Numbers state changes, merges bursts of them, and keeps the dock badge on the number of
/// approvals waiting. Cheap to clone: clones share one sequence.
#[derive(Clone)]
pub struct Notifier(Arc<NotifierInner>);

struct NotifierInner {
    output: Arc<dyn UserOutput>,
    pending_approvals: Box<dyn Fn() -> usize + Send + Sync>,
    version: AtomicU64,
    changes: Mutex<ChangeCoalescer>,
    badge: Mutex<Option<i64>>,
}

impl Notifier {
    /// `pending_approvals` counts the approvals waiting; the badge follows it.
    pub fn new(
        output: Arc<dyn UserOutput>,
        pending_approvals: impl Fn() -> usize + Send + Sync + 'static,
    ) -> Self {
        Self(Arc::new(NotifierInner {
            output,
            pending_approvals: Box::new(pending_approvals),
            version: AtomicU64::new(0),
            changes: Mutex::new(ChangeCoalescer::default()),
            badge: Mutex::new(None),
        }))
    }

    /// The version of the last change sent. A snapshot carries it, so the webview can tell
    /// whether an event it gets next is news.
    pub fn version(&self) -> u64 {
        self.0.version.load(Ordering::SeqCst)
    }

    /// Tells the webview something it mirrors has changed, merging changes that land within
    /// `CHANGE_WINDOW` of the last one into a single trailing event.
    ///
    /// The version only moves when an event goes out, so consecutive events are numbered
    /// consecutively: the webview treats a jump as a dropped event and does a full resync,
    /// which a merged burst must not look like.
    pub fn changed(&self) {
        self.notify(Change::default());
    }

    /// Like `changed`, when `origin` moved too, so the webview re-reads the repos' git state.
    pub fn repo_git_moved(&self) {
        self.notify(Change { repo_git: true });
    }

    fn notify(&self, change: Change) {
        let decision = match self.0.changes.lock() {
            Ok(mut changes) => changes.record(change, Instant::now()),
            Err(_) => Decision::EmitNow(change),
        };
        match decision {
            Decision::EmitNow(change) => self.emit(change),
            Decision::Schedule(delay) => {
                let notifier = self.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(delay).await;
                    let change = match notifier.0.changes.lock() {
                        Ok(mut changes) => changes.flush(Instant::now()),
                        Err(_) => Some(change),
                    };
                    if let Some(change) = change {
                        notifier.emit(change);
                    }
                });
            }
            Decision::Merged => {}
        }
    }

    /// A toast carries its own payload, so it needs no state change.
    pub fn toast(&self, payload: ToastRaisedPayload) {
        self.0.output.toast(payload);
    }

    pub fn toast_parts(
        &self,
        tone: ToastTone,
        parts: Vec<ToastPart>,
        workspace_id: Option<String>,
    ) {
        self.toast(ToastRaisedPayload {
            tone,
            parts,
            workspace_id,
            thread_id: None,
        });
    }

    fn emit(&self, change: Change) {
        let version = self.0.version.fetch_add(1, Ordering::SeqCst) + 1;
        self.0.output.state_changed(version, change);
        self.sync_badge();
    }

    /// Any change may have added or settled an approval, so the badge is recounted after each
    /// one, and only sent when it moved.
    fn sync_badge(&self) {
        let pending = (self.0.pending_approvals)();
        let count = (pending > 0).then_some(pending as i64);
        let Ok(mut shown) = self.0.badge.lock() else {
            return;
        };
        if *shown != count {
            *shown = count;
            self.0.output.dock_badge(count);
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Decision {
    EmitNow(Change),
    Schedule(Duration),
    /// A flush is already scheduled and will carry this change.
    Merged,
}

#[derive(Debug, Default)]
struct ChangeCoalescer {
    last_emit: Option<Instant>,
    pending: Option<Change>,
}

impl ChangeCoalescer {
    fn record(&mut self, change: Change, now: Instant) -> Decision {
        if let Some(pending) = self.pending.take() {
            self.pending = Some(pending.merge(change));
            return Decision::Merged;
        }
        match self.last_emit {
            Some(last) if now.duration_since(last) < CHANGE_WINDOW => {
                self.pending = Some(change);
                Decision::Schedule(CHANGE_WINDOW - now.duration_since(last))
            }
            _ => {
                self.last_emit = Some(now);
                Decision::EmitNow(change)
            }
        }
    }

    fn flush(&mut self, now: Instant) -> Option<Change> {
        let change = self.pending.take()?;
        self.last_emit = Some(now);
        Some(change)
    }
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub enum Emitted {
    State(Change),
    Toast(ToastRaisedPayload),
    Badge(Option<i64>),
}

#[cfg(test)]
#[derive(Default)]
pub struct RecordingOutput {
    emitted: Mutex<Vec<Emitted>>,
    versions: Mutex<Vec<u64>>,
}

#[cfg(test)]
impl RecordingOutput {
    pub fn emitted(&self) -> Vec<Emitted> {
        self.emitted.lock().unwrap().clone()
    }

    pub fn state_changes(&self) -> Vec<Change> {
        self.emitted()
            .into_iter()
            .filter_map(|event| match event {
                Emitted::State(change) => Some(change),
                _ => None,
            })
            .collect()
    }

    pub fn versions(&self) -> Vec<u64> {
        self.versions.lock().unwrap().clone()
    }

    /// Each toast's text, its parts joined.
    pub fn toasts(&self) -> Vec<String> {
        self.emitted()
            .into_iter()
            .filter_map(|event| match event {
                Emitted::Toast(payload) => Some(
                    payload
                        .parts
                        .iter()
                        .map(|part| match part {
                            ToastPart::Text { value } | ToastPart::Code { value } => value.as_str(),
                        })
                        .collect::<String>(),
                ),
                _ => None,
            })
            .collect()
    }
}

#[cfg(test)]
impl UserOutput for RecordingOutput {
    fn state_changed(&self, version: u64, change: Change) {
        self.versions.lock().unwrap().push(version);
        self.emitted.lock().unwrap().push(Emitted::State(change));
    }

    fn toast(&self, payload: ToastRaisedPayload) {
        self.emitted.lock().unwrap().push(Emitted::Toast(payload));
    }

    fn dock_badge(&self, count: Option<i64>) {
        self.emitted.lock().unwrap().push(Emitted::Badge(count));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use super::*;

    fn recorded(pending: Arc<AtomicUsize>) -> (Notifier, Arc<RecordingOutput>) {
        let recording = Arc::new(RecordingOutput::default());
        let notifier = Notifier::new(recording.clone(), move || pending.load(Ordering::SeqCst));
        (notifier, recording)
    }

    const PLAIN: Change = Change { repo_git: false };
    const REPO_GIT: Change = Change { repo_git: true };

    #[tokio::test]
    async fn a_burst_reaches_the_output_as_one_trailing_event_numbered_in_sequence() {
        let (notifier, recording) = recorded(Arc::default());
        notifier.changed();
        notifier.changed();
        notifier.repo_git_moved();
        assert_eq!(
            recording.state_changes(),
            [PLAIN],
            "the first goes out at once"
        );

        // The trailing event comes from Tauri's runtime, so wait for it rather than a fixed time.
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while recording.state_changes().len() < 2 && std::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert_eq!(recording.state_changes(), [PLAIN, REPO_GIT]);
        assert_eq!(recording.versions(), [1, 2]);
        assert_eq!(notifier.version(), 2);
    }

    #[test]
    fn the_badge_follows_the_pending_count_and_is_only_sent_when_it_moves() {
        let pending = Arc::new(AtomicUsize::new(0));
        let (notifier, recording) = recorded(pending.clone());
        notifier.emit(PLAIN);
        pending.store(2, Ordering::SeqCst);
        notifier.emit(PLAIN);
        notifier.emit(PLAIN);
        pending.store(0, Ordering::SeqCst);
        notifier.emit(PLAIN);

        let badges: Vec<_> = recording
            .emitted()
            .into_iter()
            .filter_map(|event| match event {
                Emitted::Badge(count) => Some(count),
                _ => None,
            })
            .collect();
        assert_eq!(badges, [Some(2), None]);
    }

    #[test]
    fn a_lone_change_goes_out_at_once_and_a_burst_becomes_one_trailing_event() {
        let mut changes = ChangeCoalescer::default();
        let start = Instant::now();
        assert_eq!(changes.record(PLAIN, start), Decision::EmitNow(PLAIN));
        let later = start + Duration::from_millis(10);
        assert_eq!(
            changes.record(PLAIN, later),
            Decision::Schedule(CHANGE_WINDOW - Duration::from_millis(10))
        );
        assert_eq!(changes.record(REPO_GIT, later), Decision::Merged);
        assert_eq!(changes.record(PLAIN, later), Decision::Merged);
        let flushed = start + CHANGE_WINDOW;
        assert_eq!(
            changes.flush(flushed),
            Some(REPO_GIT),
            "the merged event does everything each change asked for"
        );
        assert_eq!(changes.flush(flushed), None);
        // The flush counts as the last emit, so the next change waits again.
        assert!(matches!(
            changes.record(PLAIN, flushed),
            Decision::Schedule(_)
        ));
        let mut quiet = ChangeCoalescer::default();
        quiet.record(PLAIN, start);
        assert_eq!(
            quiet.record(PLAIN, start + CHANGE_WINDOW),
            Decision::EmitNow(PLAIN)
        );
    }
}
