import type { Thread } from '$lib/state/threads.svelte'
import { runningAgentCount } from '$lib/state/activity'
import { plural } from '$lib/format/plural'

/**
 * A workspace in a few words for its palette entry: it needs you, by the core's rule the sidebar
 * shows too, or how many agents are working.
 */
export function workspaceCardMetaText(
  needsYou: boolean,
  workspaceThreads: Thread[],
): string {
  if (needsYou) return 'Needs Attention'
  const working = Math.max(1, runningAgentCount(workspaceThreads))
  return `${plural(working, 'agent')} working`
}
