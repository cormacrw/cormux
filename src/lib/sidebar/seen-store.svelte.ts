import { readPref, writePref } from '$lib/primitives/local-pref'

const SEEN_KEY = 'cormux-sidebar-seen'

type Saved = { since: number; at: Record<string, number> }

function readSeen(): Saved | null {
  try {
    const parsed: unknown = JSON.parse(readPref(SEEN_KEY) ?? 'null')
    if (parsed == null || typeof parsed !== 'object') return null
    const { since, at } = parsed as Partial<Saved>
    if (typeof since !== 'number' || at == null || typeof at !== 'object')
      return null
    return {
      since,
      at: Object.fromEntries(
        Object.entries(at).filter(
          (entry): entry is [string, number] =>
            typeof entry[1] === 'number' && Number.isFinite(entry[1]),
        ),
      ),
    }
  } catch {
    return null
  }
}

/**
 * The activity you've caught up on, per thread: a thread is unread when it has moved since you
 * last had it open. Kept per machine, like the sidebar's other choices. The first run starts
 * from now, so the threads you already had don't all turn up unread at once.
 */
class Seen {
  #saved = readSeen()
  /** Activity up to here counts as seen for a thread you've never opened. */
  readonly since = this.#saved?.since ?? Date.now()
  at = $state<Record<string, number>>(this.#saved?.at ?? {})

  constructor() {
    if (!this.#saved) this.#save()
  }

  isUnread(id: string, activityAtMs: number): boolean {
    return activityAtMs > (this.at[id] ?? this.since)
  }

  markSeen(id: string, activityAtMs: number) {
    if (activityAtMs <= (this.at[id] ?? 0)) return
    this.at = { ...this.at, [id]: activityAtMs }
    this.#save()
  }

  #save() {
    writePref(SEEN_KEY, JSON.stringify({ since: this.since, at: this.at }))
  }
}

export const seen = new Seen()
