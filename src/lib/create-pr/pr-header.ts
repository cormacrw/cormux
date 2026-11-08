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

function resolvePrHtmlUrl(
  pr: PullRequest | undefined,
  storedUrl: string | null,
): string | null {
  return pr?.htmlUrl ?? storedUrl
}

type BranchPr = {
  number: number
  url: string | null
  /** The synced PR, when Cormux has it, for the status label. */
  synced: PullRequest | undefined
}

/**
 * The pull request for the checked-out branch, wherever it was opened: the one this
 * workspace opened, a synced open PR with this head branch, or the one gh-stack knows.
 */
export function branchPullRequest(input: {
  items: PullRequest[]
  repoId: string
  branch: string
  prNumber: number | null
  prHtmlUrl: string | null
  stackPr: { number: number; url: string | null } | null | undefined
}): BranchPr | null {
  const { items, repoId, branch, prNumber, prHtmlUrl, stackPr } = input
  if (prNumber != null) {
    const synced = prByNumber(items, repoId, prNumber)
    return {
      number: prNumber,
      url: resolvePrHtmlUrl(synced, prHtmlUrl),
      synced,
    }
  }
  const synced = prForBranch(items, repoId, branch)
  if (synced) return { number: synced.num, url: synced.htmlUrl, synced }
  if (stackPr) {
    const known = prByNumber(items, repoId, stackPr.number)
    return {
      number: stackPr.number,
      url: known?.htmlUrl ?? stackPr.url,
      synced: known,
    }
  }
  return null
}
