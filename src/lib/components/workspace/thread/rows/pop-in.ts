import type { Action } from 'svelte/action'
import { prefersReducedMotion } from '$lib/motion'
import type { EntryScope } from '$lib/thread/entering'

type PopInParams = {
  entries: EntryScope | undefined
  /** Omit for rows that animate some other way (a tool run's steps pop, a reply types in). */
  key?: string
  /** Where the pop grows from; a user bubble grows from its bottom-right corner. */
  origin?: string
}

/** Pops a row in the first time its key shows up after the thread opened. */
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
