/**
 * One subscription per key, shared by everyone watching that key and closed when the last of
 * them stops. Each key's subscription is its own: watching one never closes another.
 */
export class SharedSubscriptions {
  private open = new Map<string, { watchers: number; stop: () => void }>()

  constructor(private subscribe: (key: string) => () => void) {}

  /** Returns the function that stops this watcher; calling it again does nothing. */
  watch(key: string): () => void {
    const known = this.open.get(key)
    if (known) known.watchers += 1
    else this.open.set(key, { watchers: 1, stop: this.subscribe(key) })
    let stopped = false
    return () => {
      if (stopped) return
      stopped = true
      const entry = this.open.get(key)
      if (!entry) return
      entry.watchers -= 1
      if (entry.watchers > 0) return
      this.open.delete(key)
      entry.stop()
    }
  }

  watched(): Set<string> {
    return new Set(this.open.keys())
  }
}
