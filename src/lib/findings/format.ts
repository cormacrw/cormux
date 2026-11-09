import type { FindingRow } from '$lib/ipc/bindings'
import { FINDING_GROUPS } from './types'
import { plural } from '$lib/format/plural'

export function severitySummary(rows: Pick<FindingRow, 'severity'>[]): string {
  return FINDING_GROUPS.map(({ severity, label, one }) =>
    plural(
      rows.filter((row) => row.severity === severity).length,
      one,
      label.toLowerCase(),
    ),
  )
    .filter((line) => !line.startsWith('0 '))
    .join(', ')
}

export function selectionSummaryText(input: {
  total: number
  blocking: number
}): string {
  if (!input.total) return 'Select the findings you want fixed'
  if (!input.blocking) return `${plural(input.total, 'finding')} selected`
  return `${plural(input.total, 'finding')} selected · ${input.blocking} blocking`
}
