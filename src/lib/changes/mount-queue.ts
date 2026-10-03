const queue: Array<() => void> = []
let scheduled = false

function drain() {
  scheduled = false
  queue.shift()?.()
  if (queue.length) schedule()
}

// After the next frame paints, so each job lands in its own frame.
function schedule() {
  if (scheduled) return
  scheduled = true
  requestAnimationFrame(() => setTimeout(drain, 0))
}

/**
 * Runs `job` once a frame has painted, one job per frame, so mounting several heavy
 * views never blocks the UI in one go. Returns a cancel function.
 */
export function afterPaint(job: () => void): () => void {
  let cancelled = false
  queue.push(() => {
    if (!cancelled) job()
  })
  schedule()
  return () => {
    cancelled = true
  }
}
