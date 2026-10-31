/** `YYYY-MM-DD HH:MM:SS` from SQLite is UTC. */
export function eventCreatedAtMs(createdAt: string): number {
  if (!createdAt) return 0
  const ms = Date.parse(createdAt.replace(' ', 'T') + 'Z')
  return Number.isNaN(ms) ? 0 : ms
}

// Formatters are slow to create and every row shows a time, so these are made once.
let clock: Intl.DateTimeFormat | undefined
let stamp: Intl.DateTimeFormat | undefined

/** Clock time to the minute (`11:04 PM`). Empty when the event has no timestamp. */
export function formatThreadTime(fromMs: number): string | null {
  if (!fromMs) return null
  clock ??= new Intl.DateTimeFormat([], {
    hour: 'numeric',
    minute: '2-digit',
  })
  return clock.format(fromMs)
}

export function formatThreadTimeTitle(fromMs: number): string {
  stamp ??= new Intl.DateTimeFormat(undefined, {
    year: 'numeric',
    month: 'numeric',
    day: 'numeric',
    hour: 'numeric',
    minute: 'numeric',
    second: 'numeric',
  })
  return stamp.format(fromMs)
}
