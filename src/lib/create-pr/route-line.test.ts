import { describe, expect, it } from 'vitest'
import { formatPrRouteLine } from './route-line'

describe('formatPrRouteLine', () => {
  it('shows file counts and line deltas', () => {
    const line = formatPrRouteLine({
      branch: 'feat/auth',
      base: 'main',
      fileCount: 3,
      totals: { added: 32, deleted: 4 },
    })
    expect(line).toBe('feat/auth → main · 3 files +32 −4')
  })

  it('handles no changes', () => {
    const line = formatPrRouteLine({
      branch: 'feat/x',
      base: 'main',
      fileCount: 0,
      totals: { added: 0, deleted: 0 },
    })
    expect(line).toBe('feat/x → main · No file changes yet')
  })
})
