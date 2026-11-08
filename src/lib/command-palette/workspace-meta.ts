import type { Thread } from '$lib/state/threads.svelte'
import { plural } from '$lib/format/plural'

function workingCount(threads: Thread[]) {
  return threads.filter(
    (thread) =>
      (thread.status === 'running' || thread.status === 'provisioning') &&
      !thread.paused,
  ).length
}

/** A workspace in a few words for its palette entry: waiting on you, working, or idle. */
export function workspaceCardMetaText(workspaceThreads: Thread[]): string {
  if (workspaceThreads.some((thread) => thread.pendingApprovals > 0)) {
    return 'Needs Attention'
  }
  const working = workingCount(workspaceThreads)
  if (working > 0) return `${plural(working, 'agent')} working`
  return 'Idle'
}
