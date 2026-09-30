import type { Workspace } from '$lib/state/workspaces.svelte'
import type { Thread } from '$lib/state/threads.svelte'
import { plural } from '$lib/sidebar/status'

export type CardBadgeKind = 'needsAttention' | 'working' | 'idle'

export function workspaceCardBadgeKind(
  workspace: Workspace,
  workspaceThreads: Thread[],
): CardBadgeKind {
  const pending = workspaceThreads.reduce(
    (sum, thread) => sum + thread.pendingApprovals,
    0,
  )
  if (pending > 0 || workspace.cardStatus === 'needsAttention') {
    return 'needsAttention'
  }
  const working = workspaceThreads.filter(
    (thread) =>
      (thread.status === 'running' || thread.status === 'provisioning') &&
      !thread.paused,
  ).length
  if (working > 0) return 'working'
  return 'idle'
}

export function workspaceCardMetaText(
  workspace: Workspace,
  workspaceThreads: Thread[],
): string {
  const kind = workspaceCardBadgeKind(workspace, workspaceThreads)
  if (kind === 'needsAttention') return 'Needs Attention'
  if (kind === 'working') {
    const working = workspaceThreads.filter(
      (thread) =>
        (thread.status === 'running' || thread.status === 'provisioning') &&
        !thread.paused,
    ).length
    return `${plural(working, 'agent')} working`
  }
  return 'Idle'
}
