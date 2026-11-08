import { reuseRows } from './reuse-rows'

export type PrRelationship = 'review' | 'author' | 'mention' | 'assigned'

export type PrChecksState = 'pass' | 'fail' | 'running' | 'none'

export type PrReviewState = 'required' | 'changes' | 'approved' | 'draft'

export type PullRequest = {
  id: string
  num: number
  title: string
  author: string
  rel: PrRelationship
  head: string
  base: string
  updatedAtMs: number
  checks: PrChecksState
  failing: number | null
  review: PrReviewState
  additions: number
  deletions: number
  files: number
  isDraft: boolean
  htmlUrl: string
  repoFullName: string
  repoId: string | null
}

export class PrsStore {
  // Raw and reused: a PR sync that changed nothing leaves every reader alone.
  items = $state.raw<PullRequest[]>([])
  syncedAtMs = $state<number | null>(null)
  authConfigured = $state(false)

  hydrate(input: {
    items: PullRequest[]
    syncedAtMs: number | null
    authConfigured: boolean
  }) {
    this.items = reuseRows(this.items, input.items)
    this.syncedAtMs = input.syncedAtMs
    this.authConfigured = input.authConfigured
  }
}

export const prs = new PrsStore()
