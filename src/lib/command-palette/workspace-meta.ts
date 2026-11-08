import type { Thread } from '$lib/state/threads.svelte'
import type { Workspace } from '$lib/state/workspaces.svelte'
import { needsAttention, runningAgentCount } from '$lib/state/activity'
import { plural } from '$lib/format/plural'

/**
 * A workspace in a few words for its palette entry: it needs you, by the sidebar's rule, or how
 * many agents are working.
 */
export function workspaceCardMetaText(
  workspace: Workspace,
  workspaceThreads: Thread[],
): string {
  const live = workspaceThreads.find((thread) => !thread.retired)
  if (needsAttention(workspace, live?.status)) return 'Needs Attention'
  const working = Math.max(1, runningAgentCount(workspaceThreads))
  return `${plural(working, 'agent')} working`
}
