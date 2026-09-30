import type { PrRow } from '$lib/ipc/bindings'
import type {
  PrChecksState,
  PrReviewState,
  PullRequest,
} from '$lib/state/prs.svelte'
import type { PrRelationship } from '$lib/homebase/pr-filter'
import { parseTimestampMs } from '$lib/homebase/relative-time'

type StoredPayload = {
  num: number
  title: string
  author: string
  rel: PrRelationship
  head: string
  base: string
  updatedAt: string
  checks: PrChecksState
  failing?: number | null
  review: PrReviewState
  additions: number
  deletions: number
  files: number
  isDraft: boolean
  htmlUrl: string
  repoFullName: string
  repoId?: string | null
}

export function pullRequestFromRow(row: PrRow): PullRequest | null {
  try {
    const payload = JSON.parse(row.payload) as StoredPayload
    const updatedAtMs = parseTimestampMs(payload.updatedAt) ?? 0
    return {
      id: row.id,
      num: payload.num,
      title: payload.title,
      author: payload.author,
      rel: payload.rel,
      head: payload.head,
      base: payload.base,
      updatedAtMs,
      checks: payload.checks,
      failing: payload.failing ?? null,
      review: payload.review,
      additions: payload.additions,
      deletions: payload.deletions,
      files: payload.files,
      isDraft: payload.isDraft,
      htmlUrl: payload.htmlUrl,
      repoFullName: payload.repoFullName,
      repoId: payload.repoId ?? row.repoId ?? null,
    }
  } catch {
    return null
  }
}
