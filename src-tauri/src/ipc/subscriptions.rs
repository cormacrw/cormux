use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::watch;

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
