import { commands, type CoreError } from '$lib/ipc'
import type { StackBranch, WorkspaceStack } from '$lib/ipc/bindings'

class StacksStore {
  byWorkspace = $state<Record<string, WorkspaceStack>>({})
  loading = $state<Record<string, boolean>>({})

  get(workspaceId: string): WorkspaceStack | undefined {
    return this.byWorkspace[workspaceId]
  }

  /** The stack level that is the checked-out branch, if it's stacked. */
  branchFor(workspaceId: string, branch: string): StackBranch | undefined {
    return this.byWorkspace[workspaceId]?.branches.find(
      (level) => level.name === branch,
    )
  }

  /** Re-reads the workspace's stack; resolves why it couldn't, or null. */
  async load(workspaceId: string): Promise<CoreError | null> {
    this.loading[workspaceId] = true
    const result = await commands.getWorkspaceStack(workspaceId)
    this.loading[workspaceId] = false
    if (result.status === 'error') return result.error
    this.byWorkspace[workspaceId] = result.data
    return null
  }
}

export const stacks = new StacksStore()
