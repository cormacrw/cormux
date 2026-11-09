/** How long a field waits after the last keystroke before saving. */
const SAVE_DELAY_MS = 350

/**
 * Saves that wait for typing to pause, one pending save per key (a field, a repo's command).
 * A settings field keeps its draft beside this, so a refresh mid-typing can't overwrite it.
 */
export function debouncedSaves(delayMs = SAVE_DELAY_MS) {
  const pending = new Map<
    string,
    { timer: ReturnType<typeof setTimeout>; save: () => void }
  >()

  /** Drops a waiting save; returns whether there was one. */
  function cancel(key: string): boolean {
    const waiting = pending.get(key)
    if (!waiting) return false
    clearTimeout(waiting.timer)
    pending.delete(key)
    return true
  }

  return {
    schedule(key: string, save: () => void) {
      cancel(key)
      const timer = setTimeout(() => {
        pending.delete(key)
        save()
      }, delayMs)
      pending.set(key, { timer, save })
    },
    cancel,
    flush(key: string) {
      const waiting = pending.get(key)
      if (cancel(key)) waiting?.save()
    },
    /** For `onDestroy`: nothing typed is lost by leaving. */
    flushAll() {
      for (const key of [...pending.keys()]) this.flush(key)
    },
  }
}
