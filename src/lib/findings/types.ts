import type { FindingRow } from '$lib/ipc/bindings'

export type FindingSeverity = 'blocking' | 'suggestion' | 'nit'

export type FindingStatus = 'open' | 'sent' | 'fixed'

export const FINDING_GROUPS: {
  severity: FindingSeverity
  label: string
  note: string
}[] = [
  { severity: 'blocking', label: 'Blocking', note: 'Fix before this merges' },
  {
    severity: 'suggestion',
    label: 'Suggestions',
    note: 'Worth doing in this PR',
  },
  { severity: 'nit', label: 'Nits', note: 'Optional polish' },
]

export function isFindingSeverity(value: string): value is FindingSeverity {
  return value === 'blocking' || value === 'suggestion' || value === 'nit'
}

export function findingLocation(row: FindingRow): string {
  if (!row.file) return ''
  if (row.line != null) return `${row.file}:${row.line}`
  return row.file
}

export function openFindings(rows: FindingRow[]): FindingRow[] {
  return rows.filter((row) => row.status === 'open')
}

export function unsentOpenCount(rows: FindingRow[]): number {
  return openFindings(rows).length
}
