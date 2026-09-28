import { describe, expect, it } from 'vitest'
import { buildBranchPickerList } from './branch-picker'

describe('buildBranchPickerList', () => {
  it('orders current, repo branches, and other workspaces without duplicates', () => {
    const items = buildBranchPickerList({
      current: 'feat/auth',
      repoBranches: ['develop', 'main', 'feat/auth'],
      otherWorkspaceBranches: ['feat/billing'],
    })
    expect(items.map((item) => item.name)).toEqual([
      'feat/auth',
      'develop',
      'main',
      'feat/billing',
    ])
    expect(items[0].meta).toBe('current')
    expect(items.find((item) => item.name === 'feat/billing')?.meta).toBe(
      'inOtherWorkspace',
    )
  })
})
