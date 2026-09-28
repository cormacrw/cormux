export type Thread = {
  id: string
  workspaceId: string
  role: string
  engine: string
  status: string
  paused: boolean
  activity: string
  pendingApprovals: number
}

export class ThreadsStore {
  items = $state<Thread[]>([])

  readonly runningCount = $derived(
    this.items.filter((thread) => thread.status === 'running').length,
  )

  hydrate(items: Thread[]) {
    this.items = items
  }

  forWorkspace(workspaceId: string) {
    return this.items.filter((thread) => thread.workspaceId === workspaceId)
  }

  getById(id: string) {
    return this.items.find((thread) => thread.id === id)
  }

  setStatus(id: string, status: string) {
    this.items = this.items.map((thread) => {
      if (thread.id !== id) return thread
      const activity =
        status === 'running' ? 'Working' : status === 'idle' ? 'Idle' : thread.activity
      return { ...thread, status, paused: status === 'paused', activity }
    })
  }

  /** All threads in workspace list order, then thread list order. */
  readonly sidebarAgents = $derived.by(() => {
    return this.items
  })
}

export const threads = new ThreadsStore()
