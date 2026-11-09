import { describe, expect, it } from 'vitest'
import { formatChangeCounts } from './diff-totals'

describe('diff totals', () => {
  it('formats change counts for the Changes button', () => {
    expect(formatChangeCounts({ added: 4, deleted: 2 })).toBe('+4 −2')
    expect(formatChangeCounts({ added: 3, deleted: 0 })).toBe('+3')
    expect(formatChangeCounts({ added: 0, deleted: 0 })).toBeNull()
  })
})
