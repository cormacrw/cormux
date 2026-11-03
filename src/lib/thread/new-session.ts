import type { AgentEvent } from '$lib/ipc/bindings'
import { isLegacyNewSession } from './legacy-steps'

export const NEW_SESSION_SHORTCUT = '⌘L'

export function isNewSessionShortcut(event: KeyboardEvent) {
  return (
    (event.metaKey || event.ctrlKey) &&
    !event.shiftKey &&
    !event.altKey &&
    event.key.toLowerCase() === 'l'
  )
}

/** False when the agent has nothing to forget: no events yet, or a new session was just started. */
export function hasSessionToClear(events: { event: AgentEvent }[]) {
  const last = events.at(-1)?.event
  if (!last) return false
  if (last.type === 'step') return last.step.kind !== 'sessionStarted'
  return !isLegacyNewSession(last)
}
