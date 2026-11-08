use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use std::time::Duration;

use tauri::ipc::Channel;
use tokio::sync::{broadcast, watch};

use super::types::{AgentEvent, DiffUpdate, PtyChunk};
use crate::engines::{EngineRegistry, StreamedEvent};
use crate::state::AppState;

/// Live channel subscriptions, so the webview can stop the task feeding a channel.
#[derive(Clone, Default)]
pub struct Subscriptions {
    next_id: Arc<AtomicU32>,
    live: Arc<Mutex<HashMap<u32, watch::Sender<bool>>>>,
}

impl Subscriptions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a stream task; it should exit once the returned handle reports stopped.
    pub fn register(&self) -> SubscriptionHandle {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let (tx, rx) = watch::channel(false);
        self.live.lock().unwrap().insert(id, tx);
        SubscriptionHandle {
            id,
            rx,
            subs: self.clone(),
        }
    }

    pub fn cancel(&self, id: u32) {
        if let Some(tx) = self.live.lock().unwrap().remove(&id) {
            let _ = tx.send(true);
        }
    }

    #[cfg(test)]
    fn live_count(&self) -> usize {
        self.live.lock().unwrap().len()
    }
}

/// Owned by the stream task; dropping it unregisters the subscription.
pub struct SubscriptionHandle {
    pub id: u32,
    rx: watch::Receiver<bool>,
    subs: Subscriptions,
}

impl SubscriptionHandle {
    pub fn is_stopped(&self) -> bool {
        *self.rx.borrow() || self.rx.has_changed().is_err()
    }

    /// Resolves once the subscription is cancelled.
    pub async fn stopped(&mut self) {
        let _ = self.rx.wait_for(|stopped| *stopped).await;
    }
}

impl Drop for SubscriptionHandle {
    fn drop(&mut self) {
        self.subs.live.lock().unwrap().remove(&self.id);
    }
}

/// Feeds `channel` a thread's normalised agent events until cancelled. Returns the id to cancel by.
pub fn stream_agent_events(
    state: &AppState,
    thread_id: String,
    channel: Channel<StreamedEvent>,
) -> u32 {
    let engines = state.engines.clone();
    let mut sub = state.subscriptions.register();
    let id = sub.id;
    tauri::async_runtime::spawn(async move {
        // A new session replaces the engine, so when one closes wait for the next.
        while let Some(mut rx) = subscribe_thread(&engines, &thread_id, &mut sub).await {
            loop {
                let event = tokio::select! {
                    _ = sub.stopped() => return,
                    event = rx.recv() => event,
                };
                match event {
                    Ok(event) => {
                        let _ = channel.send(event);
                    }
                    // Every event is saved before it's sent, so the UI can read back what it missed.
                    Err(broadcast::error::RecvError::Lagged(missed)) => {
                        let _ = channel.send(AgentEvent::StreamLagged { missed }.into());
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    });
    id
}

/// Wait for the thread's engine session to exist, giving up if the subscription is cancelled
/// first. An idle thread on screen has no engine until its next prompt, so this waits on the
/// registry's spawn signal rather than polling.
async fn subscribe_thread(
    engines: &EngineRegistry,
    thread_id: &str,
    sub: &mut SubscriptionHandle,
) -> Option<broadcast::Receiver<StreamedEvent>> {
    loop {
        // Armed before the check, so a spawn landing between the two still wakes this.
        let spawned = engines.spawned().notified();
        tokio::pin!(spawned);
        spawned.as_mut().enable();
        if let Ok(rx) = engines.subscribe(thread_id) {
            return Some(rx);
        }
        tokio::select! {
            _ = sub.stopped() => return None,
            () = spawned => {}
        }
    }
}

/// Feeds `channel` a workspace's run/setup log, from the start, until cancelled.
pub fn stream_pty(state: &AppState, workspace_id: String, channel: Channel<PtyChunk>) -> u32 {
    let process = state.process.clone();
    let sub = state.subscriptions.register();
    let id = sub.id;
    tauri::async_runtime::spawn(async move {
        // Replay the log so reopening the Output tab shows what already ran.
        let mut lines = process.take_log_snapshot(&workspace_id);
        let activity = process.activity();
        let mut sub = sub;
        while !sub.is_stopped() {
            // Armed first, so a line landing just after the drain still wakes this loop.
            let woken = activity.notified();
            tokio::pin!(woken);
            woken.as_mut().enable();
            // Run and setup sessions are forwarded into the workspace log by their monitors.
            lines.extend(process.drain_pending(&workspace_id));
            for line in lines.drain(..) {
                let _ = channel.send(PtyChunk {
                    workspace_id: workspace_id.clone(),
                    line,
                });
            }
            tokio::select! {
                () = woken => {}
                () = sub.stopped() => break,
            }
            // Lets a burst of output arrive as one batch.
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    });
    id
}

/// Feeds `channel` a worktree's live diffs until cancelled.
pub async fn stream_diffs(
    state: &AppState,
    workspace_id: String,
    channel: Channel<DiffUpdate>,
) -> u32 {
    if let Err(error) = crate::git_workspace::load_workspace(state, &workspace_id).await {
        log::warn!("diff watch for {workspace_id} failed: {error}");
    }
    let diffs = state.diffs.clone();
    let mut sub = state.subscriptions.register();
    let id = sub.id;
    let subscription = diffs.subscribe(&workspace_id);
    let mut updates = diffs.updates();
    tauri::async_runtime::spawn(async move {
        let _subscription = subscription;
        let mut last = None;
        loop {
            if let Some(diff) = diffs.latest(&workspace_id)
                && !last.as_ref().is_some_and(|sent| Arc::ptr_eq(sent, &diff))
            {
                let _ = channel.send(DiffUpdate {
                    workspace_id: workspace_id.clone(),
                    diff: Some((*diff).clone()),
                });
                last = Some(diff);
            }
            tokio::select! {
                _ = updates.changed() => {}
                () = sub.stopped() => break,
            }
        }
    });
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancel_stops_and_unregisters() {
        let subs = Subscriptions::new();
        let mut handle = subs.register();
        assert!(!handle.is_stopped());
        assert_eq!(subs.live_count(), 1);
        subs.cancel(handle.id);
        assert!(handle.is_stopped());
        handle.stopped().await;
        drop(handle);
        assert_eq!(subs.live_count(), 0);
    }

    #[test]
    fn dropping_handle_unregisters() {
        let subs = Subscriptions::new();
        let handle = subs.register();
        drop(handle);
        assert_eq!(subs.live_count(), 0);
    }
}
