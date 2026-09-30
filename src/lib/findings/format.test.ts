import { describe, expect, it } from 'vitest'
import type { FindingRow } from '$lib/ipc/bindings'
import { selectionSummaryText, severitySummary } from './format'

const sample: FindingRow[] = [
  {
    id: '1',
    workspaceId: 'w',
    severity: 'blocking',
    title: 'A',
    file: 'a.ts',
    line: 1,
    explanation: 'x',
    status: 'open',
    commitSha: null,
    sentToThreadId: null,
  },
  {
    id: '2',
    workspaceId: 'w',
    severity: 'nit',
    title: 'B',
    file: null,
    line: null,
    explanation: 'y',
    status: 'open',
    commitSha: null,
    sentToThreadId: null,
  },
]

describe('findings format', () => {
  it('formats severity summary counts', () => {
    expect(severitySummary(sample)).toBe('1 blocking · 1 nit')
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
