import type { PrRow } from '$lib/ipc/bindings'
import type { PullRequest } from './prs.svelte'
import { parseTimestampMs } from '$lib/format/relative-time'

export function pullRequestFromRow(row: PrRow): PullRequest {
  const { payload } = row
  return {
    id: row.id,
    num: payload.num,
    title: payload.title,
    author: payload.author,
    rel: payload.rel,
    head: payload.head,
    base: payload.base,
    updatedAtMs: parseTimestampMs(payload.updatedAt) ?? 0,
    checks: payload.checks,
    failing: payload.failing,
    review: payload.review,
    additions: payload.additions,
    deletions: payload.deletions,
    files: payload.files,
    isDraft: payload.isDraft,
    htmlUrl: payload.htmlUrl,
    repoFullName: payload.repoFullName,
    repoId: payload.repoId ?? row.repoId,
  }
}
