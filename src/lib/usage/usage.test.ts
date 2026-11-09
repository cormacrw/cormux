import { describe, expect, it } from 'vitest'
import {
  contextPercent,
  formatCost,
  NO_USAGE,
  totalCost,
  usageDetail,
} from './usage'

describe('formatCost', () => {
  it('shows cents, and a tiny cost as under a cent', () => {
    expect(formatCost(0.42)).toBe('$0.42')
    expect(formatCost(12.3)).toBe('$12.30')
    expect(formatCost(0.0045)).toBe('<$0.01')
    expect(formatCost(0)).toBe('$0.00')
  })
})

describe('contextPercent', () => {
  it('is a whole percent of the window, and unknown without one', () => {
    expect(
      contextPercent({ usedTokens: 22_274, contextSize: 200_000, costUsd: 0 }),
    ).toBe(11)
    expect(
      contextPercent({ usedTokens: 5000, contextSize: 0, costUsd: null }),
    ).toBeNull()
  })
})

describe('usageDetail', () => {
  it('says tokens, context fill and cost, with short token counts', () => {
    expect(
      usageDetail({
        usedTokens: 22_274,
        contextSize: 1_000_000,
        costUsd: 0.42,
      }),
    ).toBe('22.3k tokens in context (2% of 1M) · $0.42 so far')
    expect(
      usageDetail({ usedTokens: 950, contextSize: 200_000, costUsd: 12.3 }),
    ).toBe('950 tokens in context (0% of 200k) · $12.30 so far')
  })

  it('says what an engine didn’t report instead of showing zero', () => {
    expect(
      usageDetail({ usedTokens: 5000, contextSize: 0, costUsd: null }),
    ).toBe('5k tokens in context · cost not reported')
    expect(usageDetail({ ...NO_USAGE, costUsd: 1 })).toBe(
      'Tokens not reported · $1.00 so far',
    )
    expect(usageDetail(NO_USAGE)).toBe('Usage not reported')
  })
})

describe('totalCost', () => {
  it('adds up the threads that reported a cost', () => {
    expect(
      totalCost([
        { ...NO_USAGE, costUsd: 0.25 },
        NO_USAGE,
        { ...NO_USAGE, costUsd: 1 },
      ]),
    ).toBe(1.25)
    expect(totalCost([NO_USAGE])).toBeNull()
  })
})
