// Indexes built inside $derived are replaced whole, never mutated, so plain Map and Set are right.
/* eslint-disable svelte/prefer-svelte-reactivity */
import type { ThreadUsage } from '$lib/usage/usage'
import { reuseRows } from './reuse-rows'
import { workspaces } from './workspaces.svelte'

export type Thread = {
  id: string
  workspaceId: string
  role: string
  engine: string
  status: string
  paused: boolean
  activity: string
  pendingApprovals: number
  usage: ThreadUsage
  /** Readable but can't be continued: a workspace's older thread, or a scratch's. */
  retired: boolean
  /** The thread that started this one, when an agent did. */
  parentThreadId?: string | null
  /** Set on a scratch's thread, which lives outside any workspace. */
  scratchId?: string
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
   * Live workspace threads only. Scratches and retired threads stay out of agent counts, and
   * so do the agents of a workspace being deleted, like the workspace itself.
   */
  readonly agents = $derived(
    this.items.filter((thread) => {
      if (thread.scratchId || thread.retired) return false
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

  /** The threads this one started. */
  childrenOf(threadId: string): Thread[] {
    return this.byParent.get(threadId) ?? NO_THREADS
  }

  /** The workspace's older threads, kept to read. */
  retiredFor(workspaceId: string) {
    return this.forWorkspace(workspaceId).filter((thread) => thread.retired)
  }

  getById(id: string) {
    return this.byId.get(id)
  }
}

export const threads = new ThreadsStore()
