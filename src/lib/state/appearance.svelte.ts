/**
 * Per-machine look and feel. Kept in localStorage (like the theme, which
 * mode-watcher owns) so index.html can apply it before first paint; keep the
 * storage keys in sync with the script there.
 */

/** Felt colours: each re-dyes the ground, sidebar, panels and primary clay (app.css). */
export const FELTS = [
  { id: 'meadow', label: 'Meadow', sidebar: '#35615B', ground: '#BFD8E6', primary: '#F08A6C' },
  { id: 'strawberry', label: 'Strawberry', sidebar: '#8A2F4E', ground: '#F6CBD3', primary: '#EF6F86' },
  { id: 'lemonade', label: 'Lemonade', sidebar: '#A84A1C', ground: '#F6E39E', primary: '#F28A4B' },
  { id: 'grape', label: 'Grape soda', sidebar: '#47398A', ground: '#D4CAF1', primary: '#E583B8' },
  { id: 'seaside', label: 'Seaside', sidebar: '#1D586D', ground: '#BEE6D6', primary: '#FF9B5E' },
] as const

export type FeltId = (typeof FELTS)[number]['id']

export const UI_FONT_SIZES = [14, 15, 16, 17, 18] as const
export const CODE_FONT_SIZES = [11, 12, 13, 14, 15, 16] as const

export const DEFAULT_UI_FONT_SIZE = 16
export const DEFAULT_CODE_FONT_SIZE = 12

const FELT_KEY = 'cormux-felt'
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
  felt = $state<FeltId>('meadow')
  uiFontSize = $state(DEFAULT_UI_FONT_SIZE)
  codeFontSize = $state(DEFAULT_CODE_FONT_SIZE)

  constructor() {
    const felt = read(FELT_KEY)
    if (FELTS.some((row) => row.id === felt)) {
      this.felt = felt as FeltId
    }
    this.uiFontSize = readSize(UI_FONT_KEY, UI_FONT_SIZES, DEFAULT_UI_FONT_SIZE)
    this.codeFontSize = readSize(
      CODE_FONT_KEY,
      CODE_FONT_SIZES,
      DEFAULT_CODE_FONT_SIZE,
    )
    this.apply()
  }

  setFelt(next: FeltId) {
    this.felt = next
    write(FELT_KEY, next)
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
    if (this.felt === 'meadow') delete root.dataset.felt
    else root.dataset.felt = this.felt
    // Tailwind sizes are rem, so the root size scales text across the UI.
    root.style.fontSize =
      this.uiFontSize === DEFAULT_UI_FONT_SIZE ? '' : `${this.uiFontSize}px`
    root.style.setProperty('--code-font-size', `${this.codeFontSize}px`)
  }
}

export const appearance = new AppearanceState()
