// Indexes built inside $derived are replaced whole, never mutated, so plain Map and Set are right.
/* eslint-disable svelte/prefer-svelte-reactivity */
import type { WorkspaceLifecycle } from '$lib/ipc/bindings'
import { reuseRows } from './reuse-rows'

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
  items = $state.raw<Workspace[]>([])
  /** Teardowns confirmed here but not yet reflected in a snapshot. */
  deletingIds = $state.raw<string[]>([])

  private readonly byId = $derived(
    new Map(this.items.map((workspace) => [workspace.id, workspace])),
  )

  /** Workspaces not being deleted; the sidebar and palette hide the rest. */
  readonly liveItems = $derived(
    this.items.filter((workspace) => !this.isDeleting(workspace)),
  )

  hydrate(items: Workspace[]) {
    this.items = reuseRows(this.items, items)
    const alive = new Set(items.map((item) => item.id))
    if (this.deletingIds.some((id) => !alive.has(id))) {
      this.deletingIds = this.deletingIds.filter((id) => alive.has(id))
    }
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
    if (!this.deletingIds.includes(id)) return
    this.deletingIds = this.deletingIds.filter((item) => item !== id)
  }

  getById(id: string) {
    return this.byId.get(id)
  }

  patchName(id: string, name: string) {
    this.items = this.items.map((item) =>
      item.id === id ? { ...item, name } : item,
    )
  }
}

export const workspaces = new WorkspacesStore()
