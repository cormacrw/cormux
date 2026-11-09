use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use tokio::sync::Mutex as AsyncMutex;

/// One worktree's git-driving calls, one at a time: two at once can trip over `index.lock`
/// or see each other's half-done work. Anything that runs git in a worktree (git itself,
/// `gh stack`) queues here.
#[derive(Default)]
pub struct WorktreeQueue {
    queues: Mutex<HashMap<String, Arc<AsyncMutex<()>>>>,
}

impl WorktreeQueue {
    pub(super) fn key(path: &Path) -> String {
        path.to_string_lossy().to_string()
    }

    pub async fn run<T, F, Fut>(&self, worktree: &Path, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        let key = Self::key(worktree);
        let lock = self
            .queues
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(key.clone())
            .or_insert_with(|| Arc::new(AsyncMutex::new(())))
            .clone();
        let result = {
            let _guard = lock.lock().await;
            f().await
        };
        // Clones are only taken under this lock, so nobody can be about to queue on it.
        let mut queues = self.queues.lock().unwrap_or_else(PoisonError::into_inner);
        if queues
            .get(&key)
            .is_some_and(|entry| Arc::ptr_eq(entry, &lock) && Arc::strong_count(entry) == 2)
        {
            queues.remove(&key);
        }
        result
    }

    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.queues.lock().unwrap().is_empty()
    }
}
