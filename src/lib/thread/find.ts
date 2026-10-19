import type { TimelineRow } from './timeline-types'

/** One match: the row it's in, and which occurrence in that row's text it is. */
export type FindMatch = { row: number; occurrence: number }

/** The key a row renders with (`data-row-key`), so a match can be found in the page. */
export function rowKey(row: TimelineRow): string {
  return row.kind === 'speaker' ? row.id : row.item.id
}

/**
 * What a row says, for searching: messages, thoughts, tool steps and their details, plan steps,
 * approvals and app events. Searching this rather than the page finds rows a long thread hasn't
 * rendered yet, and text inside collapsed steps.
 */
export function rowSearchText(row: TimelineRow): string {
  if (row.kind === 'speaker') return ''
  const item = row.item
  switch (item.kind) {
    case 'user':
    case 'thought':
      return item.text
    case 'toolRun':
      return item.steps
        .flatMap((step) =>
          step.kind === 'edit'
            ? [step.verb, step.path]
            : [step.title, step.detail ?? '', step.rawDetail ?? ''],
        )
        .join('\n')
    case 'plan':
      return item.steps.join('\n')
    case 'approval':
      return [item.title, item.what, item.why].join('\n')
    case 'event':
      return [item.title, item.detail ?? ''].join('\n')
    default:
      return ''
  }
}

function occurrences(text: string, needle: string): number {
  if (!needle) return 0
  let count = 0
  let at = text.indexOf(needle)
  while (at !== -1) {
    count += 1
    at = text.indexOf(needle, at + needle.length)
  }
  return count
}

/** Every case-insensitive match of `query`, in thread order. */
export function findMatches(rows: TimelineRow[], query: string): FindMatch[] {
  const needle = query.trim().toLocaleLowerCase()
  if (!needle) return []
  return rows.flatMap((row, index) =>
    Array.from(
      { length: occurrences(rowSearchText(row).toLocaleLowerCase(), needle) },
      (_, occurrence) => ({ row: index, occurrence }),
    ),
  )
}

/** `3 of 12`, or `No matches` once something is typed. */
export function findCounterText(current: number, total: number): string {
  if (!total) return 'No matches'
  return `${current + 1} of ${total}`
}

/** The next match index, wrapping around either end. */
export function stepMatch(current: number, total: number, by: 1 | -1): number {
  if (!total) return 0
  return (current + by + total) % total
}
