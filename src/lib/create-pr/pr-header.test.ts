import { describe, expect, it } from 'vitest'
import { primaryPrActionLabel } from './pr-header'
import type { PullRequest } from '$lib/state/prs.svelte'

const basePr: PullRequest = {
  id: 'acme/app#1',
  num: 1,
  title: 'Test',
  author: 'you',
  rel: 'author',
  head: 'feat/x',
  base: 'main',
  updatedAtMs: 0,
  checks: 'running',
  failing: null,
  review: 'required',
  additions: 1,
  deletions: 0,
  files: 1,
  isDraft: false,
  htmlUrl: 'https://github.com/acme/app/pull/1',
  repoFullName: 'acme/app',
  repoId: 'my-app',
}

describe('primaryPrActionLabel', () => {
  it('reflects check state when PR is known', () => {
    expect(primaryPrActionLabel(basePr, 1)).toBe('PR #1 · Checks running')
  })

  it('falls back when PR is not synced yet', () => {
    expect(primaryPrActionLabel(undefined, 42)).toBe('PR #42 opened')
  })
})
