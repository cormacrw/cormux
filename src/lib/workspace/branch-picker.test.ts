import { describe, expect, it } from 'vitest'
import { buildBranchPickerList } from './branch-picker'

describe('buildBranchPickerList', () => {
  it('puts the current branch first and disables what the core holds', () => {
    const items = buildBranchPickerList({
      current: 'feat/auth',
      branches: [
        { name: 'develop', hold: null },
        { name: 'feat/auth', hold: null },
        {
          name: 'feat/billing',
          hold: { kind: 'checkedOutBy', workspaces: ['Billing'] },
        },
        { name: 'main', hold: { kind: 'defaultBranch' } },
        { name: 'release', hold: { kind: 'repoHead' } },
      ],
    })
    expect(items.map((item) => item.name)).toEqual([
      'feat/auth',
      'develop',
      'feat/billing',
      'main',
      'release',
    ])
    expect(items[0]).toMatchObject({ meta: 'current', checked: true })
    expect(items.find((item) => item.name === 'develop')).toMatchObject({
      meta: null,
      disabled: false,
    })
    // Bug: the branch checked out in the repo itself was offered, and git refused it.
    expect(items.find((item) => item.name === 'release')).toMatchObject({
      meta: 'Checked out in the repo',
      disabled: true,
    })
    expect(items.find((item) => item.name === 'feat/billing')?.meta).toBe(
      'Checked out by Billing',
    )
  })
})
