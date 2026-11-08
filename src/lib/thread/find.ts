import { parseReply, replySearchText } from './reply/parse'
import type { TimelineRow } from './timeline-types'
import { stepText, workedForLabel } from './timeline/step-text'

/** One match: the row it's in, and which occurrence in that row's text it is. */
type FindMatch = { row: number; occurrence: number }

/** The key a row is drawn and tracked with (`RowContext.trackRow`). */
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
      return item.text
    case 'thought':
      return item.text
    case 'reply':
      return replySearchText(parseReply(item.text, { streaming: false }))
    case 'toolRun':
      return item.steps
        .flatMap((step) => {
          const { title, detail = '', full = '' } = stepText(step)
          return step.kind === 'tool' && step.role.kind === 'edit'
            ? [title, detail]
            : [title, detail, full]
        })
        .join('\n')
    case 'plan':
      return item.steps.join('\n')
    case 'approval':
      return [item.title, item.what, item.why].join('\n')
    case 'event':
      return [item.title, item.detail ?? ''].join('\n')
    case 'worked':
      return workedForLabel(item.startedAtMs, item.atMs)
    case 'findings':
      return ''
  }
}

// Rows keep their object while unchanged, so a search over a long thread only reads the new ones.
const searchTexts = new WeakMap<TimelineRow, string>()

function lowerSearchText(row: TimelineRow) {
  let text = searchTexts.get(row)
  if (text === undefined) {
    text = rowSearchText(row).toLocaleLowerCase()
    searchTexts.set(row, text)
  }
  return text
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
      { length: occurrences(lowerSearchText(row), needle) },
      (_, occurrence) => ({ row: index, occurrence }),
    ),
  )
}

export function findCounterText(current: number, total: number): string {
  if (!total) return 'No matches'
  return `${current + 1} of ${total}`
}

export function stepMatch(current: number, total: number, by: 1 | -1): number {
  if (!total) return 0
  return (current + by + total) % total
}
