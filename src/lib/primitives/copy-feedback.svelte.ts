const SHOW_MS = 1500

/** Copies text to the clipboard and holds which thing was copied for a moment, for a check mark. */
export class CopyFeedback {
  /** The key of what was just copied, until the check mark times out. */
  copied = $state<string | null>(null)
  #timer: ReturnType<typeof setTimeout> | undefined

  async copy(text: string, key = text) {
    try {
      await navigator.clipboard.writeText(text)
    } catch {
      return
    }
    this.copied = key
    clearTimeout(this.#timer)
    this.#timer = setTimeout(() => (this.copied = null), SHOW_MS)
  }

  /** For `onDestroy`. */
  dispose() {
    clearTimeout(this.#timer)
  }
}
