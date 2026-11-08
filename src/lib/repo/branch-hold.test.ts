import { describe, expect, it } from 'vitest'
import { branchHoldLabel } from './branch-hold'

describe('branchHoldLabel', () => {
  it('words each hold the core reports', () => {
    expect(
      branchHoldLabel({ kind: 'checkedOutBy', workspaces: ['OAuth login'] }),
    ).toBe('Checked out by OAuth login')
    expect(branchHoldLabel({ kind: 'defaultBranch' })).toBe('Default branch')
    expect(branchHoldLabel({ kind: 'repoHead' })).toBe(
      'Checked out in the repo',
    )
  })
})
