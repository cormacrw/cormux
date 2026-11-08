// Indexes built inside $derived are replaced whole, never mutated, so plain Map and Set are right.
/* eslint-disable svelte/prefer-svelte-reactivity */
import type { Snapshot, ThreadRole, ThreadStatus } from '$lib/ipc/bindings'
import type { ThreadUsage } from '$lib/usage/usage'
import { onSnapshot } from '$lib/core/sync'
import { runningAgentCount, threadActivity } from './activity'
import { reuseRows } from './reuse-rows'
import { workspaces } from './workspaces.svelte'

export type Thread = {
  id: string
  workspaceId: string
  /** Display text: `Lead`, `Reviewer`, or what an agent named it. */
  role: string
  /** Set on the workspace's own Lead or Reviewer. */
  ownRole?: ThreadRole | null
  engine: string
  status: ThreadStatus
  activity: string
  pendingApprovals: number
  usage: ThreadUsage
  /** Readable but can't be continued: a workspace's older thread. */
  retired: boolean
  /** The thread that started this one, when an agent did. */
  parentThreadId: string | null
}

/** The snapshot's workspace threads, grouped in workspace order and created order within one. */
export function threadsFromSnapshot(snapshot: Snapshot): Thread[] {
  const { persisted } = snapshot
  const pending = new Map<string, number>()
  for (const row of persisted.approvals) {
    if (row.status !== 'pending') continue
    pending.set(row.threadId, (pending.get(row.threadId) ?? 0) + 1)
  }
  const records = new Map(
    snapshot.workspaces.map((ws, index) => [ws.id, { ws, index }]),
  )
  const items = persisted.threads.map((thread): Thread => {
    const ws = records.get(thread.workspaceId)?.ws
    return {
      id: thread.id,
      workspaceId: thread.workspaceId,
      role: thread.title,
      ownRole: thread.role,
      engine: thread.engine,
      status: thread.status,
      activity: threadActivity(thread.status, ws?.activity),
      pendingApprovals: pending.get(thread.id) ?? 0,
      retired: thread.retired,
      parentThreadId: thread.parentThreadId,
      usage: {
        usedTokens: thread.usedTokens,
        contextSize: thread.contextSize,
        costUsd: thread.costUsd,
      },
    }
  })
  // The first tab is the workspace's own thread; ids are random, so sorting by id would shuffle.
  const order = (thread: Thread) => records.get(thread.workspaceId)?.index ?? 0
  return items.sort((a, b) => order(a) - order(b))
}

const NO_THREADS: Thread[] = []

export class ThreadsStore {
  items = $state.raw<Thread[]>([])

  // Lookups are indexed once per list, not scanned per caller: the sidebar, header, palette
  // and composer all ask by id or workspace on every snapshot.
  private readonly byId = $derived(
    new Map(this.items.map((thread) => [thread.id, thread])),
  )

  private readonly byWorkspace = $derived.by(() => {
    const map = new Map<string, Thread[]>()
    for (const thread of this.items) {
      const list = map.get(thread.workspaceId)
      if (list) list.push(thread)
      else map.set(thread.workspaceId, [thread])
    }
    return map
  })

  private readonly byParent = $derived.by(() => {
    const map = new Map<string, Thread[]>()
    for (const thread of this.items) {
      if (!thread.parentThreadId) continue
      const list = map.get(thread.parentThreadId)
      if (list) list.push(thread)
      else map.set(thread.parentThreadId, [thread])
    }
    return map
  })

  /**
   * Live threads only. Retired threads stay out of agent counts, and so do the agents of a
   * workspace being deleted, like the workspace itself.
   */
  readonly agents = $derived(
    this.items.filter((thread) => {
      if (thread.retired) return false
      const workspace = workspaces.getById(thread.workspaceId)
      return !workspace || !workspaces.isDeleting(workspace)
    }),
  )

  hydrate(items: Thread[]) {
    this.items = reuseRows(this.items, items)
  }

  /** The workspace's threads in creation order; the same array until the list changes. */
  forWorkspace(workspaceId: string): Thread[] {
    return this.byWorkspace.get(workspaceId) ?? NO_THREADS
  }

  /** The workspace's one thread that can still be messaged. */
  liveFor(workspaceId: string) {
    return this.forWorkspace(workspaceId).find((thread) => !thread.retired)
  }

  childrenOf(threadId: string): Thread[] {
    return this.byParent.get(threadId) ?? NO_THREADS
  }

  retiredFor(workspaceId: string) {
    return this.forWorkspace(workspaceId).filter((thread) => thread.retired)
  }

  getById(id: string) {
    return this.byId.get(id)
  }

  /** Whether the workspace's branch can change: not while an agent in it works or starts. */
  branchLock(workspaceId: string): {
    locked: boolean
    reason: string | null
    running: number
  } {
    const running = runningAgentCount(this.forWorkspace(workspaceId))
    return running > 0
      ? {
          locked: true,
          reason: 'Pause or stop the running agents to change branches.',
          running,
        }
      : { locked: false, reason: null, running }
  }
}

export const threads = new ThreadsStore()

onSnapshot((snapshot) => threads.hydrate(threadsFromSnapshot(snapshot)))
