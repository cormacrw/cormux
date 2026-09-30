import { describe, expect, it } from 'vitest'
import { branchPullRequest, primaryPrActionLabel } from './pr-header'
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

describe('branchPullRequest', () => {
  const input = {
    items: [basePr],
    repoId: 'my-app',
    branch: 'feat/y',
    prNumber: null,
    prHtmlUrl: null,
    stackPr: null,
  }

  it('prefers the PR this workspace opened', () => {
    const pr = branchPullRequest({ ...input, prNumber: 1, prHtmlUrl: 'stored' })
    expect(pr?.number).toBe(1)
    expect(pr?.url).toBe(basePr.htmlUrl)
  })

  it('finds a PR opened elsewhere by its head branch', () => {
    expect(branchPullRequest({ ...input, branch: 'feat/x' })?.number).toBe(1)
    expect(
      branchPullRequest({ ...input, branch: 'feat/x', repoId: 'other' }),
    ).toBeNull()
  })

  it('falls back to the PR gh-stack knows', () => {
    const pr = branchPullRequest({
      ...input,
      stackPr: { number: 9, url: 'https://github.com/acme/app/pull/9' },
    })
    expect(pr).toEqual({
      number: 9,
      url: 'https://github.com/acme/app/pull/9',
      synced: undefined,
    })
  })

  it('is null when the branch has no PR', () => {
    expect(branchPullRequest(input)).toBeNull()
  })
})
