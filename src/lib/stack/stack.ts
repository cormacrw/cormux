import type { DiffTarget, StackBranch, WorkspaceStack } from '$lib/ipc/bindings'
import type { PullRequest } from '$lib/state/prs.svelte'
import { plural } from '$lib/sidebar/status'

/** Top of the stack first, the way it reads on screen with the trunk underneath. */
export function stackCardsTopFirst(
  stack: WorkspaceStack | undefined,
): StackBranch[] {
  return stack ? [...stack.branches].reverse() : []
}

/** gh-stack only adds branches on top, so Add needs the top branch checked out. */
export function canAddToStack(stack: WorkspaceStack | undefined): boolean {
  if (!stack || stack.status === 'unavailable') return false
  if (stack.status === 'notStacked') return true
  return stack.branches.at(-1)?.name === stack.currentBranch
}

export function sameDiffTarget(
  a: DiffTarget | null,
  b: DiffTarget | null,
): boolean {
  return a?.head === b?.head && a?.base === b?.base
}

/**
 * Whether the diff is showing `branch` against `base`. Review workspaces diff
 * `HEAD`, which is the checked-out branch.
 */
export function isDiffTargetOf(
  target: DiffTarget | null,
  branch: string,
  base: string,
  currentBranch: string,
): boolean {
  if (!target || target.base !== base) return false
  const head = target.head === 'HEAD' ? currentBranch : target.head
  return head === branch
}

export function diffTargetLabel(target: DiffTarget | null): string {
  return target ? `${target.head} vs ${target.base}` : 'Uncommitted changes'
}

export function stackSubtitle(stack: WorkspaceStack): string {
  if (stack.status === 'unavailable')
    return 'Stacks need the gh-stack extension for the GitHub CLI.'
  if (stack.status === 'notStacked') {
    return stack.currentBranch === stack.trunk
      ? `Add a branch to start a stack on ${stack.trunk}.`
      : `${stack.currentBranch} isn't in a stack. Add a branch to start one with it at the bottom.`
  }
  const open = stack.branches.filter((branch) => !branch.merged).length
  return `${plural(open, 'open branch', 'open branches')} stacked on ${stack.trunk}`
}

export function commitsLabel(count: number): string {
  if (count === 0) return 'No commits yet'
  return plural(count, 'commit', 'commits')
}

export function prStateLabel(state: string): string {
  if (state === 'MERGED') return 'merged'
  if (state === 'QUEUED') return 'queued'
  return 'open'
}

export function pullRequestForBranch(
  items: PullRequest[],
  repoId: string,
  branch: string,
): PullRequest | undefined {
  return items.find((pr) => pr.repoId === repoId && pr.head === branch)
}
