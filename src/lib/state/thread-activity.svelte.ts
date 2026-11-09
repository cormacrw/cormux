import type { ThreadActivityRow } from '$lib/ipc/bindings'
import { onSnapshot } from '$lib/core/sync'

/** When each thread last moved, for the sidebar's order; streamed output doesn't count. */
class ThreadActivity {
  atMs = $state.raw<Record<string, number>>({})

  /** Keeps each thread's latest time, so an older snapshot can't move a thread back down. */
  merge(rows: ThreadActivityRow[]) {
    let next: Record<string, number> | null = null
    for (const row of rows) {
      if (row.atMs > (this.atMs[row.threadId] ?? 0)) {
        next ??= { ...this.atMs }
        next[row.threadId] = row.atMs
      }
    }
    if (next) this.atMs = next
  }
}

export const threadActivity = new ThreadActivity()

onSnapshot((snapshot) =>
  threadActivity.merge(snapshot.persisted.threadActivity),
)
