export type ThreadStatus = 'idle' | 'running' | 'waiting' | 'paused'

export type Thread = {
  id: string
  workspaceId: string
  title: string
  status: ThreadStatus
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
}

export const threads = new ThreadsStore()
