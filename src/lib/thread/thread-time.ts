/** `YYYY-MM-DD HH:MM:SS` from SQLite is UTC. */
export function eventCreatedAtMs(createdAt: string): number {
  if (!createdAt) return 0
  const ms = Date.parse(createdAt.replace(' ', 'T') + 'Z')
  return Number.isNaN(ms) ? 0 : ms
}

// Formatters are slow to create and every row shows a time, so these are made once, and
// each timestamp is formatted once: a row asks for its label several times per render.
let clock: Intl.DateTimeFormat | undefined
let stamp: Intl.DateTimeFormat | undefined
const clockCache = new Map<number, string>()
const stampCache = new Map<number, string>()
const CACHE_LIMIT = 4000

function remember(cache: Map<number, string>, key: number, value: string) {
  if (cache.size >= CACHE_LIMIT) cache.clear()
  cache.set(key, value)
  return value
}

/** Clock time to the minute (`11:04 PM`). Empty when the event has no timestamp. */
export function formatThreadTime(fromMs: number): string | null {
  if (!fromMs) return null
  const known = clockCache.get(fromMs)
  if (known !== undefined) return known
  clock ??= new Intl.DateTimeFormat([], {
    hour: 'numeric',
    minute: '2-digit',
  })
  return remember(clockCache, fromMs, clock.format(fromMs))
}

export function formatThreadTimeTitle(fromMs: number): string {
  const known = stampCache.get(fromMs)
  if (known !== undefined) return known
  stamp ??= new Intl.DateTimeFormat(undefined, {
    year: 'numeric',
    month: 'numeric',
    day: 'numeric',
    hour: 'numeric',
    minute: 'numeric',
    second: 'numeric',
  })
  return remember(stampCache, fromMs, stamp.format(fromMs))
}
