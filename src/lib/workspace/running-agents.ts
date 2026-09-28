import type { Thread } from '$lib/state/threads.svelte'

/** Agents that lock branch switching (spec 16 / header). */
export function runningAgentCount(threads: Thread[]): number {
  return threads.filter(
    (thread) =>
      (thread.status === 'running' && !thread.paused) ||
      thread.status === 'provisioning',
  ).length
}

export function branchPickerLocked(threads: Thread[]): boolean {
  return runningAgentCount(threads) > 0
}
