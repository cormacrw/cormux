import type { ThreadUsage } from '$lib/usage/usage'
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

export class ThreadsStore {
  items = $state<Thread[]>([])

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

  readonly runningCount = $derived(
    this.agents.filter((thread) => thread.status === 'running').length,
  )

  hydrate(items: Thread[]) {
    this.items = items
  }

  forWorkspace(workspaceId: string) {
    return this.items.filter((thread) => thread.workspaceId === workspaceId)
  }

  /** The workspace's one thread that can still be messaged. */
  liveFor(workspaceId: string) {
    return this.forWorkspace(workspaceId).find((thread) => !thread.retired)
  }

  /** The threads this one started. */
  childrenOf(threadId: string) {
    return this.items.filter((thread) => thread.parentThreadId === threadId)
  }

  /** The workspace's older threads, kept to read. */
  retiredFor(workspaceId: string) {
    return this.forWorkspace(workspaceId).filter((thread) => thread.retired)
  }

  getById(id: string) {
    return this.items.find((thread) => thread.id === id)
  }

  setStatus(id: string, status: string) {
    this.items = this.items.map((thread) => {
      if (thread.id !== id) return thread
      const activity =
        status === 'running'
          ? 'Working'
          : status === 'idle'
            ? 'Idle'
            : thread.activity
      return { ...thread, status, paused: status === 'paused', activity }
    })
  }

  /** All threads in workspace list order, then thread list order. */
  readonly sidebarAgents = $derived.by(() => {
    return this.agents
  })
}

export const threads = new ThreadsStore()
