export type WorkspaceStatus = 'idle' | 'working' | 'needsAttention'

export type Workspace = {
  id: string
  name: string
  status: WorkspaceStatus
}

export class WorkspacesStore {
  items = $state<Workspace[]>([])

  readonly workingCount = $derived(
    this.items.filter((workspace) => workspace.status === 'working').length,
  )

  readonly needsAttentionCount = $derived(
    this.items.filter((workspace) => workspace.status === 'needsAttention')
      .length,
  )

  hydrate(items: Workspace[]) {
    this.items = items
  }

  upsert(workspace: Workspace) {
    const index = this.items.findIndex((item) => item.id === workspace.id)
    if (index === -1) {
      this.items = [...this.items, workspace]
      return
    }
    this.items = this.items.map((item) =>
      item.id === workspace.id ? workspace : item,
    )
  }

  getById(id: string) {
    return this.items.find((item) => item.id === id)
  }
}

export const workspaces = new WorkspacesStore()
