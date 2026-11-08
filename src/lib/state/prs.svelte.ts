import type {
  PrChecksState,
  PrRelationship,
  PrReviewState,
} from '$lib/ipc/bindings'
import { reuseRows } from './reuse-rows'

export type { PrChecksState, PrRelationship, PrReviewState }

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
  authConfigured = $state(false)

  hydrate(input: { items: PullRequest[]; authConfigured: boolean }) {
    this.items = reuseRows(this.items, input.items)
    this.authConfigured = input.authConfigured
  }
}

export const prs = new PrsStore()
