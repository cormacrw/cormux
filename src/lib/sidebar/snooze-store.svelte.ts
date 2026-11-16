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

/** Longest a timer waits before checking again. */
const MAX_TIMER_MS = 60 * 1000

/**
 * Threads you've put away until later, by id, with when each comes back. Kept per machine, like
 * the sidebar's other choices. A snooze that has run out is forgotten, and `nowMs` ticks at that
 * moment so the thread comes back on time rather than on the next minute.
 *
 * Timers don't count time while the machine sleeps, and a hidden window's are throttled, so one
 * set for the snooze's end can fire hours late. Coming back to the window checks again, and the
 * timer never waits more than a minute, so a snooze that ran out while you were away is over by
 * the time you look.
 */
class Snoozes {
  until = $state<Record<string, number>>(readSnoozes())
  nowMs = $state(Date.now())

  #timer: ReturnType<typeof setTimeout> | undefined

  constructor() {
    this.#wake()
    if (typeof window !== 'undefined') {
      const recheck = () => this.#wake()
      window.addEventListener('focus', recheck)
      document.addEventListener('visibilitychange', () => {
        if (document.visibilityState === 'visible') recheck()
      })
    }
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
    const delay = Math.min(next - this.nowMs, MAX_TIMER_MS)
    this.#timer = setTimeout(() => this.#wake(), delay + 50)
  }
}

export const snoozes = new Snoozes()
