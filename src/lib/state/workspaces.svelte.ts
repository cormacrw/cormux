// Indexes built inside $derived are replaced whole, never mutated, so plain Map and Set are right.
/* eslint-disable svelte/prefer-svelte-reactivity */
import { onSnapshot } from '$lib/core/sync'
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

  private readonly byId = $derived(
    new Map(this.items.map((workspace) => [workspace.id, workspace])),
  )

  /** Workspaces not being deleted; the sidebar and palette hide the rest. */
  readonly liveItems = $derived(
    this.items.filter((workspace) => !this.isDeleting(workspace)),
  )

  hydrate(items: Workspace[]) {
    this.items = reuseRows(this.items, items)
  }

  isDeleting(workspace: Workspace) {
    return workspace.lifecycle === 'tearingDown'
  }

  getById(id: string) {
    return this.byId.get(id)
  }

  runtime(id: string): WorkspaceRuntime {
    return this.byId.get(id)?.runtime ?? IDLE_RUNTIME
  }
}

export const workspaces = new WorkspacesStore()

onSnapshot((snapshot) => workspaces.hydrate(workspacesFromSnapshot(snapshot)))
