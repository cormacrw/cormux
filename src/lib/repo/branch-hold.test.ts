import { describe, expect, it } from 'vitest'
import { branchHoldReason } from './branch-hold'

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
