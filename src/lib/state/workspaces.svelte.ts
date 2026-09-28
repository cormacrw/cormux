import type { WorkspaceLifecycle } from '$lib/ipc/bindings'

/** Homebase card vocabulary — sidebar uses finer-grained status via lifecycle. */
export type WorkspaceCardStatus = 'idle' | 'working' | 'needsAttention'

export type Workspace = {
  id: string
  name: string
  branch: string
  lifecycle: WorkspaceLifecycle
  paused: boolean
  activityText: string
  pendingApprovals: number
  /** Card rollup for Homebase (unchanged from earlier stub). */
  cardStatus: WorkspaceCardStatus
  createdAtMs: number | null
  summary: string | null
  summaryAtMs: number | null
  summarySource: string
  kind: 'review' | null
  prNumber: number | null
  modifiedFiles: number
  provStep: number
  setupFailedCommand: string | null
  setupFailedExitCode: number | null
}

export class WorkspacesStore {
  items = $state<Workspace[]>([])

  readonly workingCount = $derived(
    this.items.filter((workspace) => workspace.cardStatus === 'working').length,
  )

  readonly needsAttentionCount = $derived(
    this.items.filter((workspace) => workspace.cardStatus === 'needsAttention')
      .length,
  )

  /** Newest workspaces first (sidebar spec). */
  readonly sidebarItems = $derived([...this.items].reverse())

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

  patchCard(
    id: string,
    patch: Partial<
      Pick<
        Workspace,
        'summary' | 'summaryAtMs' | 'summarySource' | 'modifiedFiles'
      >
    >,
  ) {
    this.items = this.items.map((item) =>
      item.id === id ? { ...item, ...patch } : item,
    )
  }

  patchName(id: string, name: string) {
    this.items = this.items.map((item) =>
      item.id === id ? { ...item, name } : item,
    )
  }
}

export const workspaces = new WorkspacesStore()
