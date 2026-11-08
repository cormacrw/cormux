import type { AgentEvent } from '$lib/ipc/bindings'
import { isLegacyNewSession } from './legacy-steps'

/** False when the agent has nothing to forget: no events yet, or a new session was just started. */
export function hasSessionToClear(events: { event: AgentEvent }[]) {
  const last = events.at(-1)?.event
  if (!last) return false
  if (last.type === 'step') return last.step.kind !== 'sessionStarted'
  return !isLegacyNewSession(last)
}
