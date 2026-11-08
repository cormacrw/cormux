import type { TranscriptEvent } from '$lib/ipc/bindings'

/** False when the agent has nothing to forget: no events yet, or a new session was just started. */
export function hasSessionToClear(events: { event: TranscriptEvent }[]) {
  const last = events.at(-1)?.event
  if (!last) return false
  return last.type !== 'step' || last.step.kind !== 'sessionStarted'
}
