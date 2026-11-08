// Indexes built inside $derived are replaced whole, never mutated, so plain Map and Set are right.
/* eslint-disable svelte/prefer-svelte-reactivity */
import type { ReviewStatus, WorkspaceLifecycle } from '$lib/ipc/bindings'
import { reuseRows } from './reuse-rows'

/** A workspace at a glance; the sidebar reads the finer `lifecycle` instead. */
export type WorkspaceCardStatus = 'idle' | 'working' | 'needsAttention'

export type Workspace = {
  id: string
  name: string
  branch: string
  lifecycle: WorkspaceLifecycle
  paused: boolean
  activityText: string
  pendingApprovals: number
  cardStatus: WorkspaceCardStatus
  createdAtMs: number | null
  summary: string | null
  summaryAtMs: number | null
  summarySource: string
  kind: 'review' | null
  prNumber: number | null
  prHtmlUrl: string | null
  provStep: number
  setupFailedCommand: string | null
  setupFailedExitCode: number | null
  /** The workspace's own run command, which Run starts instead of its repo's. */
  runCommand: string | null
  /** Where a review of this workspace stands; null before one starts. */
  reviewStatus: ReviewStatus | null
  /** The review was posted to GitHub. */
  reviewSubmitted: boolean
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
