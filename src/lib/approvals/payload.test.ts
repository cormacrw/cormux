import { describe, expect, it } from 'vitest'
import { approvalResolvedChip, parseApprovalPayload } from './payload'

describe('parseApprovalPayload', () => {
  it('reads stored labels', () => {
    const payload = parseApprovalPayload(
      JSON.stringify({
        title: 'Delete file',
        what: 'src/a.ts',
        why: 'unused',
        okLabel: 'Approve delete',
        noLabel: 'Keep file',
      }),
    )
    expect(payload?.okLabel).toBe('Approve delete')
  })
})

describe('approvalResolvedChip', () => {
  it('formats approved chip', () => {
    const chip = approvalResolvedChip(
      {
        title: 'Delete file',
        what: 'src/a.ts',
        why: 'unused',
        okLabel: 'Approve delete',
        noLabel: 'Keep file',
        resolvedAtMs: Date.now() - 1000,
      },
      'approved',
      Date.now(),
    )
    expect(chip).toContain('Approved by you')
  })
})
