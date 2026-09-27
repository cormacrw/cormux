import type { Workspace } from '$lib/state/workspaces.svelte'
import type { Thread } from '$lib/state/threads.svelte'
import { plural } from '$lib/sidebar/status'

export function workspaceCardMetaText(
  workspace: Workspace,
  workspaceThreads: Thread[],
): string {
  const pending = workspaceThreads.reduce(
    (sum, thread) => sum + thread.pendingApprovals,
    0,
  )
  if (pending > 0 || workspace.cardStatus === 'needsAttention') {
    return 'Needs Attention'
  }
  const working = workspaceThreads.filter(
    (thread) =>
      (thread.status === 'running' || thread.status === 'provisioning') &&
      !thread.paused,
  ).length
  if (working > 0) {
    return `${plural(working, 'agent')} working`
  }
  return 'Idle'
}
