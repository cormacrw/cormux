import type { PullRequest } from './prs.svelte'

/** The synced pull request whose head is `branch` in the repo; the one rule every view uses. */
export function prForBranch(
  items: PullRequest[],
  repoId: string,
  branch: string,
): PullRequest | undefined {
  return items.find((pr) => pr.repoId === repoId && pr.head === branch)
}

export function prByNumber(
  items: PullRequest[],
  repoId: string,
  num: number,
): PullRequest | undefined {
  return items.find((pr) => pr.repoId === repoId && pr.num === num)
}
