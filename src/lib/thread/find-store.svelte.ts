/** ⌘F asks whichever conversation is on screen to open (or refocus) its find bar. */
class ThreadFindStore {
  private bars = new Set<() => void>()

  request() {
    for (const open of this.bars) open()
  }

  /** For a mounted find bar: `open` runs on each ⌘F. Returns the stop. */
  onRequest(open: () => void) {
    this.bars.add(open)
    return () => void this.bars.delete(open)
  }
}

export const threadFind = new ThreadFindStore()
