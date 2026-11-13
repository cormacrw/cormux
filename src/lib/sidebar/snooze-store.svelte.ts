import { readPref, writePref } from '$lib/primitives/local-pref'

const SNOOZE_KEY = 'cormux-sidebar-snoozed'

function readSnoozes(): Record<string, number> {
  try {
    const parsed: unknown = JSON.parse(readPref(SNOOZE_KEY) ?? '{}')
    if (parsed == null || typeof parsed !== 'object') return {}
    return Object.fromEntries(
      Object.entries(parsed).filter(
        (entry): entry is [string, number] =>
          typeof entry[1] === 'number' && Number.isFinite(entry[1]),
      ),
    )
  } catch {
    return {}
  }
}

/**
 * Threads you've put away until later, by id, with when each comes back. Kept per machine, like
 * the sidebar's other choices. A snooze that has run out is forgotten, and `nowMs` ticks at that
 * moment so the thread comes back on time rather than on the next minute.
 */
class Snoozes {
  until = $state<Record<string, number>>(readSnoozes())
  nowMs = $state(Date.now())

  #timer: ReturnType<typeof setTimeout> | undefined

  constructor() {
    this.#wake()
  }

  snooze(id: string, untilMs: number) {
    this.until = { ...this.until, [id]: untilMs }
    this.#save()
  }

  unsnooze(id: string) {
    if (!(id in this.until)) return
    const rest = { ...this.until }
    delete rest[id]
    this.until = rest
    this.#save()
  }

  #save() {
    const entries = Object.entries(this.until)
    writePref(SNOOZE_KEY, entries.length ? JSON.stringify(this.until) : null)
    this.#wake()
  }

  /** Drops what has run out and sets a timer for the next snooze to end. */
  #wake() {
    clearTimeout(this.#timer)
    this.nowMs = Date.now()
    const live = Object.entries(this.until).filter(
      ([, untilMs]) => untilMs > this.nowMs,
    )
    if (live.length !== Object.keys(this.until).length) {
      this.until = Object.fromEntries(live)
      writePref(SNOOZE_KEY, live.length ? JSON.stringify(this.until) : null)
    }
    if (live.length === 0) return
    const next = Math.min(...live.map(([, untilMs]) => untilMs))
    // Timers past ~24.8 days overflow; checking in once a day is plenty that far out.
    const delay = Math.min(next - this.nowMs, 24 * 60 * 60 * 1000)
    this.#timer = setTimeout(() => this.#wake(), delay + 50)
  }
}

export const snoozes = new Snoozes()
