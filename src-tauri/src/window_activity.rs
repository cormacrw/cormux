//! Whether the window has focus, and the sync a focus starts: every workspace's fetch and the PR
//! list, at most once a minute.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};

use crate::state::AppState;

/// How long a focus-triggered sync buys, so Cmd-Tabbing doesn't refetch every repo.
const FOCUS_SYNC_COOLDOWN: Duration = Duration::from_secs(60);

pub struct WindowActivity {
    focused: AtomicBool,
    syncing: Arc<AtomicBool>,
    last_sync: Mutex<Option<Instant>>,
}

/// A running focus sync; another can't start until it's dropped, however the sync ends.
pub struct SyncClaim(Arc<AtomicBool>);

impl Drop for SyncClaim {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// Notes the focus, and on gaining it syncs unless a sync is running or just ran.
pub fn focus_changed(app: &AppHandle, focused: bool) {
    let Some(activity) = app.try_state::<WindowActivity>() else {
        return;
    };
    activity.set_focused(focused);
    if !focused {
        return;
    }
    let Some(claim) = activity.begin_sync(Instant::now()) else {
        return;
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let _claim = claim;
        let state = app.state::<AppState>();
        crate::workspace_git::fetch_all(&state).await;
        if let Err(error) = state.pr_sync.sync_app(&state).await {
            log::warn!("focus PR sync failed: {error}");
        }
    });
}

impl WindowActivity {
    pub fn new() -> Self {
        Self {
            focused: AtomicBool::new(true),
            syncing: Arc::new(AtomicBool::new(false)),
            last_sync: Mutex::new(None),
        }
    }

    pub fn is_focused(&self) -> bool {
        self.focused.load(Ordering::Relaxed)
    }

    pub fn set_focused(&self, focused: bool) {
        self.focused.store(focused, Ordering::Relaxed);
    }

    /// Refused while a sync runs or just ran.
    fn begin_sync(&self, now: Instant) -> Option<SyncClaim> {
        let mut last = self.last_sync.lock().ok()?;
        if self.syncing.load(Ordering::SeqCst)
            || last.is_some_and(|at| now.duration_since(at) < FOCUS_SYNC_COOLDOWN)
        {
            return None;
        }
        self.syncing.store(true, Ordering::SeqCst);
        *last = Some(now);
        Some(SyncClaim(self.syncing.clone()))
    }
}

impl Default for WindowActivity {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_syncs_once_per_cooldown_and_never_overlap() {
        let activity = WindowActivity::new();
        let start = Instant::now();
        let later = start + FOCUS_SYNC_COOLDOWN;

        let claim = activity.begin_sync(start);
        assert!(claim.is_some());
        assert!(activity.begin_sync(later).is_none(), "one is still running");
        drop(claim);
        assert!(
            activity
                .begin_sync(start + Duration::from_secs(1))
                .is_none()
        );
        assert!(activity.begin_sync(later).is_some());
    }

    #[test]
    fn a_sync_that_panics_still_lets_the_next_one_run() {
        let activity = WindowActivity::new();
        let start = Instant::now();
        let claim = activity.begin_sync(start).unwrap();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _claim = claim;
            panic!("sync failed");
        }));
        assert!(activity.begin_sync(start + FOCUS_SYNC_COOLDOWN).is_some());
    }
}
