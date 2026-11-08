import { describe, expect, it } from 'vitest'
import { dockBadgeCount, pendingApprovalCount } from './toast-payload'

describe('pendingApprovalCount', () => {
  it('counts only pending rows', () => {
    expect(
      pendingApprovalCount([
        { status: 'pending' },
        { status: 'resolved' },
        { status: 'pending' },
      ]),
    ).toBe(2)
  })

  it('includes live broker counts', () => {
    expect(pendingApprovalCount([], 2)).toBe(2)
  })
})

describe('dockBadgeCount', () => {
  it('clears the badge at zero', () => {
    expect(dockBadgeCount(0)).toBeUndefined()
  })

  it('passes through positive counts', () => {
    expect(dockBadgeCount(3)).toBe(3)
  })
})
