import type { TimelineRow } from './timeline-types'

/**
 * Tells rows added while a thread is on screen from rows that were already there, so only the
 * new ones animate. Opening a thread, remounting a row, or scrolling a virtual list never does.
 */
export type EntryScope = {
  /** True once per key, and only for keys that weren't in the thread when it opened. */
  claim(key: string): boolean
}

export function createEntryScope(rows: TimelineRow[]): EntryScope {
  const known = new Set<string>()
  for (const row of rows) {
    if (row.kind === 'speaker') {
      known.add(row.id)
      continue
    }
    known.add(row.item.id)
    if (row.item.kind === 'toolRun') {
      for (const step of row.item.steps) known.add(step.id)
    }
  }
  return {
    claim(key) {
      if (known.has(key)) return false
      known.add(key)
      return true
    },
  }
}
