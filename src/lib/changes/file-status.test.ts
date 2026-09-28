import { describe, expect, it } from 'vitest'
import { inferFileStatus } from './file-status'

describe('inferFileStatus', () => {
  it('marks deletions', () => {
    expect(
      inferFileStatus({
        path: 'gone.ts',
        added: 0,
        deleted: 4,
        hunks: [{ header: '', body: '-x\n' }],
      }),
    ).toBe('D')
  })

  it('marks additions', () => {
    expect(
      inferFileStatus({
        path: 'new.ts',
        added: 3,
        deleted: 0,
        hunks: [{ header: '', body: '+x\n' }],
      }),
    ).toBe('A')
  })
})
