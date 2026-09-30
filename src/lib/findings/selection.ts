import type { FindingRow } from '$lib/ipc/bindings'
import type { FindingSeverity } from './types'
import { openFindings } from './types'

export type QuickSelectMode = 'blocking' | 'all' | 'none'

export function emptySelection(): Set<string> {
  return new Set()
}

export function defaultSelectedIds(rows: FindingRow[]): Set<string> {
  const selected = new Set<string>()
  for (const row of rows) {
    if (row.status === 'open' && row.severity === 'blocking') {
      selected.add(row.id)
    }
  }
  return selected
}

export function mergeSelection(
  rows: FindingRow[],
  previous: Set<string>,
  touched: boolean,
): Set<string> {
  if (touched) return new Set(previous)
  return defaultSelectedIds(rows)
}

export function toggleFinding(
  selected: Set<string>,
  id: string,
  checked: boolean,
): Set<string> {
  const next = new Set(selected)
  if (checked) next.add(id)
  else next.delete(id)
  return next
}

export function groupCheckboxState(
  rows: FindingRow[],
  selected: Set<string>,
  severity: FindingSeverity,
): { checked: boolean; indeterminate: boolean; disabled: boolean } {
  const open = openFindings(rows).filter((row) => row.severity === severity)
  if (!open.length) {
    return { checked: false, indeterminate: false, disabled: true }
  }
  const on = open.filter((row) => selected.has(row.id)).length
  return {
    checked: on === open.length,
    indeterminate: on > 0 && on < open.length,
    disabled: false,
  }
}

export function setGroupSelection(
  rows: FindingRow[],
  selected: Set<string>,
  severity: FindingSeverity,
  checked: boolean,
): Set<string> {
  const next = new Set(selected)
  for (const row of openFindings(rows)) {
    if (row.severity !== severity) continue
    if (checked) next.add(row.id)
    else next.delete(row.id)
  }
  return next
}

export function applyQuickSelect(
  rows: FindingRow[],
  selected: Set<string>,
  mode: QuickSelectMode,
): Set<string> {
  const open = openFindings(rows)
  if (mode === 'none') return new Set()
  if (mode === 'all') return new Set(open.map((row) => row.id))
  const next = new Set<string>()
  for (const row of open) {
    if (row.severity === 'blocking') next.add(row.id)
  }
  return next
}

export function selectedSummary(
  rows: FindingRow[],
  selected: Set<string>,
): { total: number; blocking: number } {
  const open = openFindings(rows)
  const picked = open.filter((row) => selected.has(row.id))
  return {
    total: picked.length,
    blocking: picked.filter((row) => row.severity === 'blocking').length,
  }
}

export function rowCheckboxState(
  row: FindingRow,
  selected: Set<string>,
): { checked: boolean; disabled: boolean } {
  if (row.status === 'sent' || row.status === 'fixed') {
    return { checked: true, disabled: true }
  }
  return { checked: selected.has(row.id), disabled: false }
}
