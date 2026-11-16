/**
 * Attention: the workspace needs you, by the core's rule (`attention.needsYou`). Quiet: it's
 * working, or it's a read-only leftover there's nothing to do with.
 */
export type ThreadUrgency = 'attention' | 'quiet'

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
  const children = new Map<string, T[]>()
  for (const item of items) {
    if (item.parentId == null || !ids.has(item.parentId)) continue
    const list = children.get(item.parentId)
    if (list) list.push(item)
    else children.set(item.parentId, [item])
  }
  const nested: (T & { nested: boolean })[] = []
  for (const item of items) {
    if (item.parentId != null && ids.has(item.parentId)) continue
    nested.push({ ...item, nested: false })
    for (const child of children.get(item.id) ?? []) {
      nested.push({ ...child, nested: true })
    }
  }
  return nested
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

export type SendAndNextMode = 'newestUnread' | 'oldest'

/**
 * Where ⌘↵ goes after sending: another thread that needs you, never the one you're on. `items`
 * are the sidebar's Threads section, already sorted, so snoozed and working threads aren't in
 * it. Oldest is the one that has waited longest (the top of the list); newest unread is the
 * unread one that moved last. Null when there's nowhere to go.
 */
export function nextThreadAfterSend<
  T extends {
    id: string
    urgency: ThreadUrgency
    activityAtMs: number
    workspace: unknown
  },
>(
  items: T[],
  currentId: string,
  mode: SendAndNextMode,
  isUnread: (item: T) => boolean,
): T | null {
  const waiting = items.filter(
    (item) =>
      item.id !== currentId &&
      item.workspace != null &&
      item.urgency === 'attention',
  )
  if (mode === 'oldest') {
    return waiting.reduce<T | null>(
      (best, item) =>
        best == null || item.activityAtMs < best.activityAtMs ? item : best,
      null,
    )
  }
  return waiting
    .filter(isUnread)
    .reduce<T | null>(
      (best, item) =>
        best == null || item.activityAtMs > best.activityAtMs ? item : best,
      null,
    )
}
