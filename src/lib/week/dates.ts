/** Week maths on local calendar days, written as `YYYY-MM-DD`. Weeks start on Monday. */

export type Day = string

const pad = (value: number) => String(value).padStart(2, '0')

/** The local calendar day of `date`. */
export function isoDay(date: Date): Day {
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
}

/** Today's local calendar day. */
export function today(): Day {
  return isoDay(new Date())
}

/** SQLite's `datetime('now')`: UTC, `YYYY-MM-DD HH:MM:SS`. */
export function sqliteNow(ms = Date.now()) {
  return new Date(ms).toISOString().slice(0, 19).replace('T', ' ')
}

/** Local midnight on `day`. */
export function parseDay(day: Day): Date {
  const [year, month, date] = day.split('-').map(Number)
  return new Date(year!, month! - 1, date!)
}

export function addDays(day: Day, count: number): Day {
  const date = parseDay(day)
  date.setDate(date.getDate() + count)
  return isoDay(date)
}

/** The Monday on or before `day`. */
export function weekStart(day: Day): Day {
  const offset = (parseDay(day).getDay() + 6) % 7
  return addDays(day, -offset)
}

/** Monday to Sunday of the week starting `start`. */
export function weekDays(start: Day): Day[] {
  return Array.from({ length: 7 }, (_, index) => addDays(start, index))
}

const MONTH = new Intl.DateTimeFormat('en-US', { month: 'short' })
const WEEKDAY = new Intl.DateTimeFormat('en-US', { weekday: 'short' })
const LONG_WEEKDAY = new Intl.DateTimeFormat('en-US', { weekday: 'long' })

/** `Oct 19 – 25`, or `Oct 26 – Nov 1` when the week crosses a month. */
export function weekRange(start: Day): string {
  const first = parseDay(start)
  const last = parseDay(addDays(start, 6))
  const from = `${MONTH.format(first)} ${first.getDate()}`
  return first.getMonth() === last.getMonth()
    ? `${from} – ${last.getDate()}`
    : `${from} – ${MONTH.format(last)} ${last.getDate()}`
}

/** `{ name: 'Mon', date: 'Oct 19', long: 'Monday' }` for a lane header. */
export function dayLabel(day: Day) {
  const date = parseDay(day)
  return {
    name: WEEKDAY.format(date),
    long: LONG_WEEKDAY.format(date),
    date: `${MONTH.format(date)} ${date.getDate()}`,
  }
}

/** The local day a SQLite `datetime('now')` stamp (UTC, `YYYY-MM-DD HH:MM:SS`) falls on. */
export function localDayOf(stamp: string | null | undefined): Day | null {
  if (!stamp) return null
  const ms = Date.parse(`${stamp.replace(' ', 'T')}Z`)
  return Number.isNaN(ms) ? null : isoDay(new Date(ms))
}
