import { onSnapshot } from '$lib/core/sync'

/** The workspaces that need you, by the core's rule (`workspaces::needs_attention`). */
class Attention {
  private ids = $state.raw<ReadonlySet<string>>(new Set())

  hydrate(ids: string[]) {
    this.ids = new Set(ids)
  }

  needsYou(workspaceId: string): boolean {
    return this.ids.has(workspaceId)
  }
}

export const attention = new Attention()

onSnapshot((snapshot) => attention.hydrate(snapshot.workspacesNeedingAttention))
