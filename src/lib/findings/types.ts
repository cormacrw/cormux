import type { FindingRow } from '$lib/ipc/bindings'

export type FindingSeverity = 'blocking' | 'suggestion' | 'nit'

export const FINDING_GROUPS: {
  severity: FindingSeverity
  label: string
  /** One of them, in a count: `1 suggestion`. */
  one: string
  note: string
}[] = [
  {
    severity: 'blocking',
    label: 'Blocking',
    one: 'blocking',
    note: 'Fix before this merges',
  },
  {
    severity: 'suggestion',
    label: 'Suggestions',
    one: 'suggestion',
    note: 'Worth doing in this PR',
  },
  { severity: 'nit', label: 'Nits', one: 'nit', note: 'Optional polish' },
]

export function findingLocation(row: FindingRow): string {
  if (!row.file) return ''
  if (row.line != null) return `${row.file}:${row.line}`
  return row.file
}

export function openFindings(rows: FindingRow[]): FindingRow[] {
  return rows.filter((row) => row.status === 'open')
}
