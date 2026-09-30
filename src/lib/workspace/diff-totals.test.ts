import { describe, expect, it } from 'vitest'
import { formatChangeCounts, totalsFromDiffFiles } from './diff-totals'

describe('diff totals', () => {
  it('sums added and deleted lines', () => {
    expect(
      totalsFromDiffFiles([
        { path: 'a.ts', added: 3, deleted: 1, hunks: [] },
        { path: 'b.ts', added: 2, deleted: 0, hunks: [] },
      ]),
    ).toEqual({ added: 5, deleted: 1 })
  })

  it('formats change counts for the Changes button', () => {
    expect(formatChangeCounts({ added: 4, deleted: 2 })).toBe('+4 −2')
    expect(formatChangeCounts({ added: 0, deleted: 0 })).toBeNull()
  })
})
