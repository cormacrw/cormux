import type { AgentEvent, WorkspaceLifecycle } from '$lib/ipc/bindings'

/**
 * Attention: nothing is happening, so the thread needs prompting or closing. Quiet: it's working,
 * or it's a read-only leftover there's nothing to do with.
 */
export type ThreadUrgency = 'attention' | 'quiet'

/**
 * Whether an event moved the thread: a message from you or the end of a turn. Streamed output
 * doesn't count, so a working thread holds its place in the list instead of jumping on every chunk.
 */
export function isThreadActivity(event: AgentEvent): boolean {
  return (
    event.type === 'turnEnd' ||
    event.type === 'engineExited' ||
    (event.type === 'messageChunk' && event.role === 'user')
  )
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
 * A thread's children sit right under it. One whose parent isn't in this list stays where it is,
 * since it has nothing to sit under.
 */
export function nestThreadList<
  T extends { id: string; parentId: string | null },
>(items: T[]): (T & { nested: boolean })[] {
  const ids = new Set(items.map((item) => item.id))
  const isChild = (item: T) => item.parentId != null && ids.has(item.parentId)
  return items
    .filter((item) => !isChild(item))
    .flatMap((item) => [
      { ...item, nested: false },
      ...items
        .filter((child) => isChild(child) && child.parentId === item.id)
        .map((child) => ({ ...child, nested: true })),
    ])
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
