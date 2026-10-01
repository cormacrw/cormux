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
  prHtmlUrl: string | null
  modifiedFiles: number
  provStep: number
  setupFailedCommand: string | null
  setupFailedExitCode: number | null
}

export class WorkspacesStore {
  items = $state<Workspace[]>([])
  /** Teardowns confirmed here but not yet reflected in a snapshot. */
  deletingIds = $state<string[]>([])

  readonly workingCount = $derived(
    this.items.filter((workspace) => workspace.cardStatus === 'working').length,
  )

  readonly needsAttentionCount = $derived(
    this.items.filter((workspace) => workspace.cardStatus === 'needsAttention')
      .length,
  )

  /** Workspaces not being deleted; the sidebar and palette hide the rest. */
  readonly liveItems = $derived(
    this.items.filter((workspace) => !this.isDeleting(workspace)),
  )

  /** Newest workspaces first (sidebar spec). */
  readonly sidebarItems = $derived([...this.liveItems].reverse())

  hydrate(items: Workspace[]) {
    this.items = items
    this.deletingIds = this.deletingIds.filter((id) =>
      items.some((item) => item.id === id),
    )
  }

  isDeleting(workspace: Workspace) {
    return (
      workspace.lifecycle === 'tearingDown' ||
      this.deletingIds.includes(workspace.id)
    )
  }

  markDeleting(id: string) {
    if (this.deletingIds.includes(id)) return
    this.deletingIds = [...this.deletingIds, id]
  }

  clearDeleting(id: string) {
    this.deletingIds = this.deletingIds.filter((item) => item !== id)
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

  patchName(id: string, name: string) {
    this.items = this.items.map((item) =>
      item.id === id ? { ...item, name } : item,
    )
  }
}

export const workspaces = new WorkspacesStore()
