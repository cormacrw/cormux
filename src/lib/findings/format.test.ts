import { describe, expect, it } from 'vitest'
import type { FindingRow } from '$lib/ipc/bindings'
import { selectionSummaryText, severitySummary } from './format'

const severities = (...list: FindingRow['severity'][]) =>
  list.map((severity) => ({ severity }))

describe('findings format', () => {
  it('counts each severity present, most severe first', () => {
    expect(severitySummary(severities('nit', 'blocking'))).toBe(
      '1 blocking, 1 nit',
    )
    expect(severitySummary(severities('suggestion', 'suggestion'))).toBe(
      '2 suggestions',
    )
  })

  it('formats selection summary', () => {
    expect(selectionSummaryText({ total: 0, blocking: 0 })).toBe(
      'Select the findings you want fixed',
    )
    expect(selectionSummaryText({ total: 3, blocking: 2 })).toBe(
      '3 findings selected · 2 blocking',
    )
  })
})
