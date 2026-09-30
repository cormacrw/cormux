/**
 * Per-machine look and feel. Kept in localStorage (like the theme, which
 * mode-watcher owns) so index.html can apply it before first paint; keep the
 * storage keys in sync with the script there.
 */

export const ACCENTS = [
  { id: 'neutral', label: 'Graphite' },
  { id: 'blue', label: 'Blue' },
  { id: 'violet', label: 'Violet' },
  { id: 'green', label: 'Green' },
  { id: 'gold', label: 'Gold' },
  { id: 'orange', label: 'Orange' },
  { id: 'rose', label: 'Rose' },
] as const

export type AccentId = (typeof ACCENTS)[number]['id']

export const UI_FONT_SIZES = [14, 15, 16, 17, 18] as const
export const CODE_FONT_SIZES = [11, 12, 13, 14, 15, 16] as const

export const DEFAULT_UI_FONT_SIZE = 16
export const DEFAULT_CODE_FONT_SIZE = 12

const ACCENT_KEY = 'cormux-accent'
const UI_FONT_KEY = 'cormux-ui-font-size'
const CODE_FONT_KEY = 'cormux-code-font-size'

function read(key: string): string | null {
  try {
    return localStorage.getItem(key)
  } catch {
    return null
  }
}

function write(key: string, value: string) {
  try {
    localStorage.setItem(key, value)
  } catch {
    // Storage can be unavailable; the choice still applies for this session.
  }
}

function readSize(key: string, allowed: readonly number[], fallback: number) {
  const value = Number(read(key))
  return allowed.includes(value) ? value : fallback
}

class AppearanceState {
  accent = $state<AccentId>('neutral')
  uiFontSize = $state(DEFAULT_UI_FONT_SIZE)
  codeFontSize = $state(DEFAULT_CODE_FONT_SIZE)

  constructor() {
    const accent = read(ACCENT_KEY)
    if (ACCENTS.some((row) => row.id === accent)) {
      this.accent = accent as AccentId
    }
    this.uiFontSize = readSize(UI_FONT_KEY, UI_FONT_SIZES, DEFAULT_UI_FONT_SIZE)
    this.codeFontSize = readSize(
      CODE_FONT_KEY,
      CODE_FONT_SIZES,
      DEFAULT_CODE_FONT_SIZE,
    )
    this.apply()
  }

  setAccent(next: AccentId) {
    this.accent = next
    write(ACCENT_KEY, next)
    this.apply()
  }

  setUiFontSize(next: number) {
    if (!UI_FONT_SIZES.includes(next as never)) return
    this.uiFontSize = next
    write(UI_FONT_KEY, String(next))
    this.apply()
  }

  setCodeFontSize(next: number) {
    if (!CODE_FONT_SIZES.includes(next as never)) return
    this.codeFontSize = next
    write(CODE_FONT_KEY, String(next))
    this.apply()
  }

  private apply() {
    if (typeof document === 'undefined') return
    const root = document.documentElement
    if (this.accent === 'neutral') delete root.dataset.accent
    else root.dataset.accent = this.accent
    // Tailwind sizes are rem, so the root size scales text across the UI.
    root.style.fontSize =
      this.uiFontSize === DEFAULT_UI_FONT_SIZE ? '' : `${this.uiFontSize}px`
    root.style.setProperty('--code-font-size', `${this.codeFontSize}px`)
  }
}

export const appearance = new AppearanceState()
