// Indexes built inside $derived are replaced whole, never mutated, so plain Map and Set are right.
/* eslint-disable svelte/prefer-svelte-reactivity */
import type { WorkspaceAppStatus } from '$lib/ipc/bindings'
import { onSnapshot } from '$lib/core/sync'
import { deepEqual } from '$lib/deep-equal'
import { reuseRows } from './reuse-rows'
import {
  IDLE_RUNTIME,
  workspacesFromSnapshot,
  type Workspace,
  type WorkspaceRuntime,
} from './workspace-model'

export type { Workspace, WorkspaceRuntime }

export class WorkspacesStore {
  // Raw and reused: a snapshot that changed nothing here changes nothing downstream.
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

  runtime(id: string): WorkspaceRuntime {
    return this.byId.get(id)?.runtime ?? IDLE_RUNTIME
  }

  /** Branches the repo's other workspaces have checked out, which this one can't take. */
  branchesInUse(repoId: string, exceptId: string): string[] {
    return this.items
      .filter(
        (workspace) =>
          workspace.repoId === repoId &&
          workspace.id !== exceptId &&
          workspace.branch,
      )
      .map((workspace) => workspace.branch)
  }

  patchName(id: string, name: string) {
    this.patch(id, (workspace) => ({ ...workspace, name }))
  }

  setAppStatus(
    id: string,
    appStatus: WorkspaceAppStatus,
    port: number | null = null,
    exitCode: number | null = null,
  ) {
    const current = this.runtime(id)
    const nextPort =
      appStatus === 'stopped' || appStatus === 'crashed'
        ? null
        : (port ?? current.port)
    this.patchRuntime(id, {
      appStatus,
      port: nextPort,
      https: nextPort === null ? false : current.https,
      exitCode: appStatus === 'crashed' ? (exitCode ?? current.exitCode) : null,
    })
  }

  clearLog(id: string) {
    this.patchRuntime(id, { logVersion: this.runtime(id).logVersion + 1 })
  }

  private patchRuntime(id: string, patch: Partial<WorkspaceRuntime>) {
    this.patch(id, (workspace) => {
      const runtime = { ...workspace.runtime, ...patch }
      return deepEqual(runtime, workspace.runtime)
        ? workspace
        : { ...workspace, runtime }
    })
  }

  private patch(id: string, update: (workspace: Workspace) => Workspace) {
    const current = this.byId.get(id)
    if (!current) return
    const next = update(current)
    if (next === current) return
    this.items = this.items.map((item) => (item.id === id ? next : item))
  }
}

export const workspaces = new WorkspacesStore()

onSnapshot((snapshot) =>
  workspaces.hydrate(
    workspacesFromSnapshot(snapshot, (id) => workspaces.getById(id)?.runtime),
  ),
)
