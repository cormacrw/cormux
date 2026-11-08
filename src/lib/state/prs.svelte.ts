import type { PrChecksState, PrReviewState } from '$lib/ipc/bindings'
import type { PrRow } from '$lib/ipc/bindings'
import { onSnapshot } from '$lib/core/sync'
import { parseTimestampMs } from '$lib/format/relative-time'
import { prForBranch } from './pr-lookup'
import { reuseRows } from './reuse-rows'

export type { PrChecksState, PrReviewState }

export type PullRequest = {
  id: string
  num: number
  title: string
  author: string
  head: string
  base: string
  updatedAtMs: number
  checks: PrChecksState
  review: PrReviewState
  files: number
  isDraft: boolean
  htmlUrl: string
  repoFullName: string
  repoId: string | null
}

export class PrsStore {
  // Raw and reused: a PR sync that changed nothing leaves every reader alone.
  items = $state.raw<PullRequest[]>([])
  authConfigured = $state(false)

  hydrate(input: { items: PullRequest[]; authConfigured: boolean }) {
    this.items = reuseRows(this.items, input.items)
    this.authConfigured = input.authConfigured
  }

  forBranch(repoId: string, branch: string) {
    return prForBranch(this.items, repoId, branch)
  }

  /** The pull request `repoFullName#num`, if Cormux knows it. */
  find(repoFullName: string, num: number) {
    return this.items.find(
      (pr) => pr.repoFullName === repoFullName && pr.num === num,
    )
  }
}

function pullRequestFromRow(row: PrRow): PullRequest {
  const { payload } = row
  return {
    id: row.id,
    num: payload.num,
    title: payload.title,
    author: payload.author,
    head: payload.head,
    base: payload.base,
    updatedAtMs: parseTimestampMs(payload.updatedAt) ?? 0,
    checks: payload.checks,
    review: payload.review,
    files: payload.files,
    isDraft: payload.isDraft,
    htmlUrl: payload.htmlUrl,
    repoFullName: payload.repoFullName,
    repoId: payload.repoId ?? row.repoId,
  }
}

export const prs = new PrsStore()

onSnapshot((snapshot) =>
  prs.hydrate({
    items: snapshot.persisted.pullRequests.map(pullRequestFromRow),
    authConfigured: snapshot.githubAuthConfigured,
  }),
)
