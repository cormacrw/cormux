/** ⌘F asks whichever conversation is on screen to open (or refocus) its find bar. */
export class ThreadFindStore {
  /** Bumped on each ⌘F; the visible find bar watches it. */
  requests = $state(0)

  request() {
    this.requests += 1
  }
}

export const threadFind = new ThreadFindStore()
