import { describe, expect, it } from 'vitest'
import {
  dockBadgeCount,
  formatToastPayload,
  pendingApprovalCount,
  pullToast,
  workspaceAppToast,
} from './toast-payload'

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

describe('toast payloads', () => {
  it('formats app running copy', () => {
    const payload = workspaceAppToast('demo', 'run', 3000, 'ws-1')
    expect(formatToastPayload(payload)).toBe(
      'demo is running on localhost:3000',
    )
    expect(payload.tone).toBe('ok')
  })

  it('formats pull copy with plural commits', () => {
    const payload = pullToast('demo', 2, 'main', 'ws-1')
    expect(formatToastPayload(payload)).toContain('2')
    expect(formatToastPayload(payload)).toContain('main')
  })
})
