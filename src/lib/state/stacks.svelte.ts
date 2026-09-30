import { commands } from '$lib/ipc'
import type { WorkspaceStack } from '$lib/ipc/bindings'
import { toastCoreError } from '$lib/feedback/wire-feedback'

export class StacksStore {
  byWorkspace = $state<Record<string, WorkspaceStack>>({})
  loading = $state<Record<string, boolean>>({})

  get(workspaceId: string): WorkspaceStack | undefined {
    return this.byWorkspace[workspaceId]
  }

  async load(workspaceId: string) {
    this.loading[workspaceId] = true
    const result = await commands.getWorkspaceStack(workspaceId)
    this.loading[workspaceId] = false
    if (result.status === 'ok') {
      this.byWorkspace[workspaceId] = result.data
    } else {
      toastCoreError(result.error)
    }
  }
}

export const stacks = new StacksStore()
