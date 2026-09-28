import type { PullRequest } from '$lib/state/prs.svelte'

export function pullRequestForWorkspace(
  items: PullRequest[],
  repoId: string,
  prNumber: number | null,
): PullRequest | undefined {
  if (prNumber == null) return undefined
  return items.find(
    (row) => row.num === prNumber && (row.repoId === repoId || row.repoId == null),
  )
}

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

export function resolvePrHtmlUrl(
  pr: PullRequest | undefined,
  storedUrl: string | null,
): string | null {
  return pr?.htmlUrl ?? storedUrl
}
