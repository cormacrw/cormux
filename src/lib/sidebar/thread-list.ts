import type { AgentEvent, WorkspaceLifecycle } from '$lib/ipc/bindings'

/**
 * Attention: nothing is happening, so the thread needs prompting or closing. Quiet: it's working,
 * or it's a read-only leftover there's nothing to do with.
 */
export type ThreadUrgency = 'attention' | 'quiet'

type TimedEvent = { atMs: number; event: AgentEvent }

/**
 * When the thread last moved: a message from you or the end of a turn. Streamed output doesn't
 * count, so a working thread holds its place in the list instead of jumping on every chunk.
 */
export function threadActivityMs(events: TimedEvent[]): number {
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const row = events[index]
    if (!row) continue
    const { event } = row
    if (
      event.type === 'turnEnd' ||
      event.type === 'engineExited' ||
      (event.type === 'messageChunk' && event.role === 'user')
    ) {
      return row.atMs
    }
  }
  return 0
}

export function threadUrgency(input: {
  lifecycle: WorkspaceLifecycle
  paused: boolean
  pendingApprovals: number
  /** The thread itself: sending marks it at once, before the workspace hears. */
  threadStatus: string | undefined
}): ThreadUrgency {
  const working =
    input.threadStatus === 'running' ||
    input.threadStatus === 'provisioning' ||
    input.lifecycle === 'creating' ||
    input.lifecycle === 'provisioning' ||
    (input.lifecycle === 'running' && !input.paused)
  return working && input.pendingApprovals === 0 ? 'quiet' : 'attention'
}

/**
 * Working threads step out of the way into their own section, except the one you're on: it stays
 * put under you, and only moves once you leave it. Read-only leftovers are quiet but not working.
 */
export function splitThreadList<
  T extends { id: string; urgency: ThreadUrgency; workspace: unknown },
>(items: T[], currentId: string | null): { threads: T[]; working: T[] } {
  const isParked = (item: T) =>
    item.urgency === 'quiet' && item.workspace != null && item.id !== currentId
  return {
    threads: items.filter((item) => !isParked(item)),
    working: items.filter(isParked),
  }
}

/**
 * Whatever needs you first, with what has waited longest on top. Below that, most recent
 * activity first. Ties keep their order, so rows don't swap for no reason.
 */
export function sortThreadList<
  T extends { urgency: ThreadUrgency; activityAtMs: number },
>(items: T[]): T[] {
  return [...items].sort((a, b) => {
    if (a.urgency !== b.urgency) return a.urgency === 'attention' ? -1 : 1
    return a.urgency === 'attention'
      ? a.activityAtMs - b.activityAtMs
      : b.activityAtMs - a.activityAtMs
  })
}
