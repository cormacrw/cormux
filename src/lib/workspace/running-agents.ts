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

export function branchLockTooltip(running: number): string {
  if (running <= 0) return ''
  if (running === 1) {
    return 'Pause or stop the running agent to switch branches.'
  }
  return `Pause or stop the ${running} running agents to switch branches.`
}
