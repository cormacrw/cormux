import { describe, expect, it } from 'vitest'
import { branchPullRequest, primaryPrActionLabel } from './branch-pr'
import type { PullRequest } from '$lib/state/prs.svelte'

const basePr: PullRequest = {
  id: 'acme/app#1',
  num: 1,
  title: 'Test',
  author: 'you',
  head: 'feat/x',
  base: 'main',
  updatedAtMs: 0,
  checks: 'running',
  review: 'required',
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
    opened: null,
    stackPr: null,
  }

  it('prefers the PR this workspace opened', () => {
    const pr = branchPullRequest({
      ...input,
      opened: { number: 1, url: 'stored' },
    })
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
      stackPr: {
        number: 9,
        url: 'https://github.com/acme/app/pull/9',
        state: 'merged',
      },
    })
    expect(pr).toEqual({
      number: 9,
      url: 'https://github.com/acme/app/pull/9',
      state: 'merged',
      synced: undefined,
    })
  })

  it('prefers the synced PR over the one gh-stack knows, with its state', () => {
    const pr = branchPullRequest({
      ...input,
      branch: 'feat/x',
      stackPr: { number: 1, url: null, state: 'queued' },
    })
    expect(pr?.url).toBe(basePr.htmlUrl)
    expect(pr?.state).toBe('queued')
  })

  it('is null when the branch has no PR', () => {
    expect(branchPullRequest(input)).toBeNull()
  })
})
