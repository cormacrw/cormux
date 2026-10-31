export type BatchScheduler = {
  frame: (run: () => void) => number
  cancelFrame: (id: number) => void
  timer: (run: () => void, ms: number) => ReturnType<typeof setTimeout>
  cancelTimer: (id: ReturnType<typeof setTimeout>) => void
}

const browserScheduler: BatchScheduler = {
  frame: (run) => requestAnimationFrame(run),
  cancelFrame: (id) => cancelAnimationFrame(id),
  timer: (run, ms) => setTimeout(run, ms),
  cancelTimer: (id) => clearTimeout(id),
}

// Frames stop in a hidden or occluded window; events must still land.
const FALLBACK_MS = 100

/** Collects what arrives within a frame and hands it over in one call, in arrival order. */
export function createEventBatcher<T>(
  onFlush: (batch: T[]) => void,
  scheduler: BatchScheduler = browserScheduler,
) {
  let pending: T[] = []
  let frame: number | null = null
  let timer: ReturnType<typeof setTimeout> | null = null

  function flush() {
    if (frame !== null) scheduler.cancelFrame(frame)
    if (timer !== null) scheduler.cancelTimer(timer)
    frame = null
    timer = null
    if (!pending.length) return
    const batch = pending
    pending = []
    onFlush(batch)
  }

  return {
    push(item: T) {
      pending.push(item)
      if (frame !== null) return
      frame = scheduler.frame(flush)
      timer = scheduler.timer(flush, FALLBACK_MS)
    },
    flush,
  }
}
