import type { AgentEvent } from '$lib/ipc/bindings'

/** Written by the core's `new_thread_session`; the timeline shows it as a divider. */
export const NEW_SESSION_TITLE = 'Started a new session'

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
  return !(last.type === 'toolCall' && last.title === NEW_SESSION_TITLE)
}
