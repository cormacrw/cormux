import { describe, expect, it } from 'vitest'
import type { FindingRow } from '$lib/ipc/bindings'
import {
  applyQuickSelect,
  defaultSelectedIds,
  groupCheckboxState,
  selectedSummary,
  setGroupSelection,
  toggleFinding,
} from './selection'

function row(
  partial: Partial<FindingRow> & Pick<FindingRow, 'id' | 'severity' | 'status'>,
): FindingRow {
  return {
    workspaceId: 'w1',
    title: partial.title ?? 'Title',
    file: partial.file ?? 'a.ts',
    line: partial.line ?? 1,
    explanation: partial.explanation ?? 'Because',
    commitSha: null,
    sentToThreadId: null,
    ...partial,
  }
}

describe('findings selection', () => {
  const rows = [
    row({ id: 'b1', severity: 'blocking', status: 'open' }),
    row({ id: 'b2', severity: 'blocking', status: 'sent' }),
    row({ id: 's1', severity: 'suggestion', status: 'open' }),
    row({ id: 'n1', severity: 'nit', status: 'open' }),
  ]

  it('selects open blocking findings by default', () => {
    expect([...defaultSelectedIds(rows)]).toEqual(['b1'])
  })

  it('toggles and groups selection on open rows only', () => {
    let selected = defaultSelectedIds(rows)
    selected = toggleFinding(selected, 's1', true)
    expect(selected.has('s1')).toBe(true)
    const group = groupCheckboxState(rows, selected, 'suggestion')
    expect(group.checked).toBe(true)
    selected = setGroupSelection(rows, selected, 'suggestion', false)
    expect(selected.has('s1')).toBe(false)
  })

  it('applies quick select modes', () => {
    expect([...applyQuickSelect(rows, new Set(), 'blocking')]).toEqual(['b1'])
    expect([...applyQuickSelect(rows, new Set(), 'all')].sort()).toEqual([
      'b1',
      'n1',
      's1',
    ])
    expect([...applyQuickSelect(rows, defaultSelectedIds(rows), 'none')]).toEqual(
      [],
    )
  })

  it('summarises selected blocking count', () => {
    const selected = new Set(['b1', 's1'])
    expect(selectedSummary(rows, selected)).toEqual({ total: 2, blocking: 1 })
  })
})
