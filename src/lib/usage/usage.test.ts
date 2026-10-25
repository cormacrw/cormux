import { describe, expect, it } from 'vitest'
import {
  costSince,
  startOfDayMs,
  contextPercent,
  formatCost,
  formatTokens,
  NO_USAGE,
  totalCost,
  usageSummary,
} from './usage'

describe('formatting', () => {
  it('shortens token counts', () => {
    expect(formatTokens(950)).toBe('950')
    expect(formatTokens(22_274)).toBe('22.3k')
    expect(formatTokens(40_000)).toBe('40k')
    expect(formatTokens(1_200_000)).toBe('1.2M')
  })

  it('shows cents, and a tiny cost as under a cent', () => {
    expect(formatCost(0.42)).toBe('$0.42')
    expect(formatCost(12.3)).toBe('$12.30')
    expect(formatCost(0.0045)).toBe('<$0.01')
    expect(formatCost(0)).toBe('$0.00')
  })
})

describe('usageSummary', () => {
  it('shows tokens, context fill and cost', () => {
    const usage = { usedTokens: 22_274, contextSize: 1_000_000, costUsd: 0.42 }
    expect(contextPercent(usage)).toBe(2)
    expect(usageSummary(usage)).toEqual({
      meter: '22.3k · 2% · $0.42',
      detail: '22.3k tokens in context (2% of 1M) · $0.42 so far',
    })
  })

  it('says what an engine didn’t report instead of showing zero', () => {
    expect(
      usageSummary({ usedTokens: 5000, contextSize: 0, costUsd: null }),
    ).toEqual({
      meter: '5k',
      detail: '5k tokens in context · cost not reported',
    })
    expect(usageSummary(NO_USAGE)).toEqual({
      meter: null,
      detail: 'Usage not reported',
    })
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

describe('costSince', () => {
  const usage = (atMs: number, cost: number | null) => ({
    atMs,
    event: { type: 'usage', cost_usd: cost },
  })

  it('counts only what the running total grew by since then', () => {
    expect(
      costSince([usage(100, 1.5), usage(200, 2), usage(300, 2.75)], 150),
    ).toBeCloseTo(1.25)
  })

  it('is null when nothing was spent since then', () => {
    expect(costSince([usage(100, 1.5)], 150)).toBeNull()
    expect(costSince([usage(200, null)], 150)).toBeNull()
  })

  it('counts a thread that started since then in full', () => {
    expect(costSince([usage(200, 0.4)], 150)).toBeCloseTo(0.4)
  })

  it('finds local midnight', () => {
    const noon = new Date(2026, 9, 24, 12, 30).getTime()
    expect(startOfDayMs(noon)).toBe(new Date(2026, 9, 24).getTime())
  })
})
