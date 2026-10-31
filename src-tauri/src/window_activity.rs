use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// How long a focus-triggered sync buys, so Cmd-Tabbing doesn't refetch every repo.
const FOCUS_SYNC_COOLDOWN: Duration = Duration::from_secs(60);

/// Whether the window has focus, and whether focusing it should sync again yet.
pub struct WindowActivity {
    focused: AtomicBool,
    syncing: AtomicBool,
    last_sync: Mutex<Option<Instant>>,
}

impl WindowActivity {
    pub fn new() -> Self {
        Self {
            focused: AtomicBool::new(true),
            syncing: AtomicBool::new(false),
            last_sync: Mutex::new(None),
        }
    }

    pub fn is_focused(&self) -> bool {
        self.focused.load(Ordering::Relaxed)
    }

    pub fn set_focused(&self, focused: bool) {
        self.focused.store(focused, Ordering::Relaxed);
    }

    /// Claims the sync; the caller must `finish_sync`. Refused while one runs or just ran.
    pub fn begin_sync(&self, now: Instant) -> bool {
        let Ok(mut last) = self.last_sync.lock() else {
            return false;
        };
        if self.syncing.load(Ordering::SeqCst)
            || last.is_some_and(|at| now.duration_since(at) < FOCUS_SYNC_COOLDOWN)
        {
            return false;
        }
        self.syncing.store(true, Ordering::SeqCst);
        *last = Some(now);
        true
    }

    pub fn finish_sync(&self) {
        self.syncing.store(false, Ordering::SeqCst);
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

        assert!(activity.begin_sync(start));
        assert!(!activity.begin_sync(later), "one is still running");
        activity.finish_sync();
        assert!(!activity.begin_sync(start + Duration::from_secs(1)));
        assert!(activity.begin_sync(later));
    }
}
