import { LruCache } from './memo'

// Formatters are slow to create and every row shows a time, so these are made once, and
// each timestamp is formatted once: a row asks for its label several times per render.
let clock: Intl.DateTimeFormat | undefined
let stamp: Intl.DateTimeFormat | undefined
const clocks = new LruCache<number, string>(4000)
const stamps = new LruCache<number, string>(4000)

/** Clock time to the minute (`11:04 PM`). Empty when the event has no timestamp. */
export function formatThreadTime(fromMs: number): string {
  if (!fromMs) return ''
  clock ??= new Intl.DateTimeFormat([], {
    hour: 'numeric',
    minute: '2-digit',
  })
  return clocks.get(fromMs) ?? clocks.set(fromMs, clock.format(fromMs))
}

/** The full date and time, for a tooltip. Empty when the event has no timestamp. */
export function formatThreadTimeTitle(fromMs: number): string {
  if (!fromMs) return ''
  stamp ??= new Intl.DateTimeFormat(undefined, {
    year: 'numeric',
    month: 'numeric',
    day: 'numeric',
    hour: 'numeric',
    minute: 'numeric',
    second: 'numeric',
  })
  return stamps.get(fromMs) ?? stamps.set(fromMs, stamp.format(fromMs))
}
