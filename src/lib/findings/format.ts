import type { FindingRow } from '$lib/ipc/bindings'
import { FINDING_GROUPS } from './types'

export function severitySummary(rows: FindingRow[]): string {
  return FINDING_GROUPS.map(({ severity, label }) => {
    const count = rows.filter((row) => row.severity === severity).length
    const word =
      count === 1 ? label.toLowerCase().replace(/s$/, '') : label.toLowerCase()
    return `${count} ${word}`
  })
    .filter((line) => !line.startsWith('0 '))
    .join(' · ')
}

export function pluralFindings(count: number, adjective?: string): string {
  const word = count === 1 ? 'finding' : 'findings'
  if (!adjective) return `${count} ${word}`
  return `${count} ${adjective} ${word}`
}

export function selectionSummaryText(input: {
  total: number
  blocking: number
}): string {
  if (!input.total) return 'Select the findings you want fixed'
  if (!input.blocking) return `${pluralFindings(input.total)} selected`
  return `${pluralFindings(input.total)} selected · ${input.blocking} blocking`
}
