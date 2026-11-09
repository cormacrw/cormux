/**
 * Where Backdrop's wordmark sits. It sinks out of view while a conversation is on screen,
 * which the thread on screen reports, and comes back with the greeting. It only animates
 * the move when one is asked for: a first prompt sent, or a conversation cleared.
 */

/** The glide's two moves; Backdrop's CSS takes them from here. */
export const GLIDE_RISE_MS = 1200
export const GLIDE_FALL_MS = 420
const GLIDE_MS = Math.max(GLIDE_RISE_MS, GLIDE_FALL_MS)

class WordmarkState {
  /** The thread on screen shows a conversation rather than its greeting. */
  private conversationShowing = $state(false)
  /** A new session is on its way: rise now rather than once its greeting lands. */
  private risingEarly = $state(false)
  /** The wordmark animates to its new place instead of jumping there. */
  gliding = $state(false)
  private glideTimer: ReturnType<typeof setTimeout> | undefined

  readonly away = $derived(this.conversationShowing && !this.risingEarly)

  /** For the thread on screen, each time what it shows changes. */
  showConversation(showing: boolean) {
    this.conversationShowing = showing
    if (!showing) this.risingEarly = false
  }

  /** Animates the next move: a conversation starting or being cleared. */
  glide() {
    this.gliding = true
    clearTimeout(this.glideTimer)
    this.glideTimer = setTimeout(() => {
      this.gliding = false
    }, GLIDE_MS)
  }

  /** Glides back up ahead of a conversation being cleared; returns the undo for when it isn't. */
  riseEarly(): () => void {
    this.glide()
    this.risingEarly = true
    return () => {
      this.risingEarly = false
    }
  }

  /** Back in place without animating, as when leaving a thread. */
  reset() {
    clearTimeout(this.glideTimer)
    this.gliding = false
    this.conversationShowing = false
    this.risingEarly = false
  }
}

export const wordmark = new WordmarkState()
