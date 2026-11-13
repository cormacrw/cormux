const HOUR_MS = 60 * 60 * 1000

/** Snoozes that run to a morning wake you at this hour, local time. */
const MORNING_HOUR = 9

export type SnoozeOption = {
  id: 'hour' | 'three-hours' | 'tomorrow' | 'next-week'
  label: string
  /** When a snooze picked at `nowMs` ends. */
  until: (nowMs: number) => number
}

function morning(nowMs: number, daysAhead: number): number {
  const date = new Date(nowMs)
  date.setDate(date.getDate() + daysAhead)
  date.setHours(MORNING_HOUR, 0, 0, 0)
  return date.getTime()
}

/** Next Monday morning; on a Monday, the one a week out. */
function nextMonday(nowMs: number): number {
  const day = new Date(nowMs).getDay()
  return morning(nowMs, (8 - day) % 7 || 7)
}

export const SNOOZE_OPTIONS: readonly SnoozeOption[] = [
  { id: 'hour', label: '1 hour', until: (nowMs) => nowMs + HOUR_MS },
  {
    id: 'three-hours',
    label: '3 hours',
    until: (nowMs) => nowMs + 3 * HOUR_MS,
  },
  { id: 'tomorrow', label: 'Tomorrow', until: (nowMs) => morning(nowMs, 1) },
  { id: 'next-week', label: 'Next week', until: nextMonday },
]

/** Snoozed threads leave the list for their own section until their time is up. */
export function splitSnoozed<T extends { id: string }>(
  items: T[],
  snoozedUntil: Readonly<Record<string, number>>,
  nowMs: number,
): { awake: T[]; snoozed: T[] } {
  const isSnoozed = (item: T) => (snoozedUntil[item.id] ?? 0) > nowMs
  return {
    awake: items.filter((item) => !isSnoozed(item)),
    snoozed: items.filter(isSnoozed),
  }
}

/**
 * When a snooze ends, said the way you'd say it: a time today, a weekday this week, or a date
 * beyond that.
 */
export function formatSnoozeUntil(
  untilMs: number,
  nowMs: number,
  { short = false }: { short?: boolean } = {},
): string {
  const until = new Date(untilMs)
  const time = until.toLocaleTimeString(undefined, {
    hour: 'numeric',
    minute: '2-digit',
  })
  const startOfDay = (ms: number) => new Date(ms).setHours(0, 0, 0, 0)
  // Rounded, so a daylight saving change in between doesn't shift the day.
  const days = Math.round(
    (startOfDay(untilMs) - startOfDay(nowMs)) / (24 * HOUR_MS),
  )
  if (days <= 0) return time
  // Short, for a sidebar row: just the day, which is what tells snoozes apart.
  if (short)
    return days < 7
      ? until.toLocaleDateString(undefined, { weekday: 'short' })
      : until.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
  if (days === 1) return `Tomorrow ${time}`
  if (days < 7)
    return `${until.toLocaleDateString(undefined, { weekday: 'short' })} ${time}`
  return `${until.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })} ${time}`
}
