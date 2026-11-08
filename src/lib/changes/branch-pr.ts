import type { StackPrState, StackPullRequest } from '$lib/ipc/bindings'
import type { PullRequest } from '$lib/state/prs.svelte'
import { prByNumber, prForBranch } from '$lib/state/pr-lookup'

export function primaryPrActionLabel(
  pr: PullRequest | undefined,
  prNumber: number,
): string {
  if (!pr) return `PR #${prNumber} opened`
  if (pr.review === 'approved') return `PR #${prNumber} · Approved`
  if (pr.checks === 'fail') return `PR #${prNumber} · Checks failed`
  if (pr.checks === 'running') return `PR #${prNumber} · Checks running`
  if (pr.review === 'changes') return `PR #${prNumber} · Changes requested`
  if (pr.isDraft) return `Draft PR #${prNumber}`
  return `View PR #${prNumber}`
}

export type BranchPr = {
  number: number
  url: string | null
  state: StackPrState
  /** The synced PR, when Cormux has it, for the status label. */
  synced: PullRequest | undefined
}

/**
 * The pull request for a branch, wherever it was opened: the one the workspace opened from it,
 * a synced open PR with this head branch, or the one gh-stack knows.
 */
export function branchPullRequest(input: {
  items: PullRequest[]
  repoId: string
  branch: string
  /** The PR the workspace opened, when `branch` is the one it opened it from. */
  opened: { number: number; url: string | null } | null
  stackPr: StackPullRequest | null | undefined
}): BranchPr | null {
  const { items, repoId, branch, opened, stackPr } = input
  const stateOf = (number: number): StackPrState =>
    stackPr?.number === number ? stackPr.state : 'open'
  if (opened) {
    const synced = prByNumber(items, repoId, opened.number)
    return {
      number: opened.number,
      url: synced?.htmlUrl ?? opened.url,
      state: stateOf(opened.number),
      synced,
    }
  }
  const synced = prForBranch(items, repoId, branch)
  if (synced) {
    return {
      number: synced.num,
      url: synced.htmlUrl,
      state: stateOf(synced.num),
      synced,
    }
  }
  if (stackPr) {
    const known = prByNumber(items, repoId, stackPr.number)
    return {
      number: stackPr.number,
      url: known?.htmlUrl ?? stackPr.url,
      state: stackPr.state,
      synced: known,
    }
  }
  return null
}
