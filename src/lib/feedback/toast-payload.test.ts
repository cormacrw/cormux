import { describe, expect, it } from 'vitest'
import { pendingApprovalCount } from './toast-payload'

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
