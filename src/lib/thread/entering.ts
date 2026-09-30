import type { Action } from 'svelte/action'
import type { TimelineItem, TimelineRow } from './timeline-types'

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

/** The agent reply still being written: the one right before the live row, if any. */
export function liveReplyId(items: TimelineItem[]): string | null {
  if (items.at(-1)?.kind !== 'live') return null
  const last = items.at(-2)
  return last?.kind === 'thought' && last.role === 'agent' ? last.id : null
}

export function prefersReducedMotion() {
  return (
    document.documentElement.classList.contains('reduce-motion') ||
    window.matchMedia('(prefers-reduced-motion: reduce)').matches
  )
}

export type PopInParams = {
  entries: EntryScope | undefined
  /** Omit for rows that animate some other way (a tool run's steps pop, a reply types in). */
  key?: string
  /** Where the pop grows from; a user bubble grows from its bottom-right corner. */
  origin?: string
}

export const popIn: Action<HTMLElement, PopInParams> = (
  node,
  { entries, key, origin = 'left center' },
) => {
  if (!key || !entries?.claim(key) || prefersReducedMotion()) return
  node.style.transformOrigin = origin
  node.animate(
    [
      { opacity: 0, transform: 'translateY(8px) scale(0.94)' },
      { opacity: 1, transform: 'none' },
    ],
    // Overshoots a little, so it lands with a pop rather than a slide.
    { duration: 320, easing: 'cubic-bezier(0.34, 1.4, 0.64, 1)' },
  )
}
