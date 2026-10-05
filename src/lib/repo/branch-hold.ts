import type { PullRequest } from '$lib/state/prs.svelte'

/** Why a local branch cannot be deleted, or null when the button should work. */
export function branchHoldReason(input: {
  name: string
  defaultBranch: string
  head: string
  checkedOutBy: string[]
}): string | null {
  if (input.checkedOutBy.length > 0) {
    return `Checked out by ${input.checkedOutBy.join(', ')}`
  }
  if (input.name === input.defaultBranch) return 'Default branch'
  if (input.head && input.name === input.head) return 'Checked out in the repo'
  return null
}

export function pullRequestForBranch(
  prs: PullRequest[],
  repoId: string,
  branch: string,
): PullRequest | null {
  return prs.find((pr) => pr.repoId === repoId && pr.head === branch) ?? null
}
