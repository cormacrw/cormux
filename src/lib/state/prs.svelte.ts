import {
  filterPullRequests,
  prFilterCounts,
  type PrFilter,
  type PrRelationship,
} from '$lib/homebase/pr-filter'

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
  items = $state<PullRequest[]>([])
  filter = $state<PrFilter>('all')
  syncedAtMs = $state<number | null>(null)
  authConfigured = $state(false)

  readonly count = $derived(this.items.length)

  readonly filtered = $derived(filterPullRequests(this.items, this.filter))

  readonly filterCounts = $derived(prFilterCounts(this.items))

  readonly isEmpty = $derived(this.items.length === 0)

  readonly filterEmpty = $derived(
    this.items.length > 0 && this.filtered.length === 0,
  )

  hydrate(input: {
    items: PullRequest[]
    syncedAtMs: number | null
    authConfigured: boolean
  }) {
    this.items = input.items
    this.syncedAtMs = input.syncedAtMs
    this.authConfigured = input.authConfigured
  }

  setFilter(filter: PrFilter) {
    this.filter = filter
  }
}

export const prs = new PrsStore()
