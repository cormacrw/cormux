import type { FindingRow } from '$lib/ipc/bindings'
import { FINDING_GROUPS } from './types'
import { plural } from '$lib/format/plural'

export function severitySummary(rows: FindingRow[]): string {
  return FINDING_GROUPS.map(({ severity, label }) => {
    const count = rows.filter((row) => row.severity === severity).length
    const word =
      count === 1 ? label.toLowerCase().replace(/s$/, '') : label.toLowerCase()
    return `${count} ${word}`
  })
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
