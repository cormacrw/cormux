import { describe, expect, it } from 'vitest'
import type { PullRequest } from '$lib/state/prs.svelte'
import { branchHoldReason, pullRequestForBranch } from './branch-hold'

describe('branchHoldReason', () => {
  const base = { defaultBranch: 'main', head: 'main', checkedOutBy: [] }

  it('holds a workspace checkout, the default branch, and the repo checkout', () => {
    expect(
      branchHoldReason({
        ...base,
        name: 'feat/oauth-login',
        checkedOutBy: ['OAuth login'],
      }),
    ).toBe('Checked out by OAuth login')
    expect(branchHoldReason({ ...base, name: 'main' })).toBe('Default branch')
    expect(
      branchHoldReason({ ...base, name: 'feat/wip', head: 'feat/wip' }),
    ).toBe('Checked out in the repo')
    expect(branchHoldReason({ ...base, name: 'feat/colors' })).toBe(null)
  })
})

describe('pullRequestForBranch', () => {
  it('matches a pull request on this repo by head branch', () => {
    const pr = { repoId: 'my-app', head: 'feat/colors', num: 12 } as PullRequest
    expect(pullRequestForBranch([pr], 'my-app', 'feat/colors')).toBe(pr)
    expect(pullRequestForBranch([pr], 'other', 'feat/colors')).toBe(null)
    expect(pullRequestForBranch([pr], 'my-app', 'main')).toBe(null)
  })
})
