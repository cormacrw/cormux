import { commands, type CoreError } from '$lib/ipc'
import type { StackBranch, WorkspaceStack } from '$lib/ipc/bindings'
import { latestRequests } from '$lib/core/owner-scoped'

class StacksStore {
  byWorkspace = $state<Record<string, WorkspaceStack>>({})
  loading = $state<Record<string, boolean>>({})

  get(workspaceId: string): WorkspaceStack | undefined {
    return this.byWorkspace[workspaceId]
  }

  branchFor(workspaceId: string, branch: string): StackBranch | undefined {
    return this.byWorkspace[workspaceId]?.branches.find(
      (level) => level.name === branch,
    )
  }

  private readonly loads = latestRequests()

  /** Reads the stack again; a reply that a newer read overtook is dropped. */
  async load(workspaceId: string): Promise<CoreError | null> {
    const isLatest = this.loads(workspaceId)
    this.loading[workspaceId] = true
    const result = await commands.getWorkspaceStack(workspaceId)
    if (!isLatest()) return null
    this.loading[workspaceId] = false
    if (result.status === 'error') return result.error
    this.byWorkspace[workspaceId] = result.data
    return null
  }
}

export const stacks = new StacksStore()
