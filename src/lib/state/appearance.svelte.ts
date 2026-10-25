/**
 * Per-machine look and feel. Kept in localStorage (like the theme, which
 * mode-watcher owns) so index.html can apply it before first paint; keep the
 * storage keys in sync with the script there.
 */

/**
 * Colour themes. Neutral is grey glass with a blue primary; the others tint
 * the window, the glass and the primary with one bold hue. The hues live in
 * app.css (`data-palette` on <html>); `hue` and `strength` (--tint-s,
 * default 1) here only draw the picker's previews, so keep them in sync.
 */
export const THEMES = [
  { id: 'neutral', label: 'Neutral', hue: null },
  { id: 'blue', label: 'Blue', hue: 262 },
  { id: 'violet', label: 'Violet', hue: 292 },
  { id: 'pink', label: 'Pink', hue: 345 },
  { id: 'red', label: 'Red', hue: 22, strength: 0.5 },
  { id: 'orange', label: 'Orange', hue: 50, strength: 0.5 },
  { id: 'green', label: 'Green', hue: 155, strength: 0.35 },
  { id: 'teal', label: 'Teal', hue: 200 },
] as const

export type ThemeId = (typeof THEMES)[number]['id']

export const DEFAULT_THEME: ThemeId = 'blue'

export const UI_FONT_SIZES = [14, 15, 16, 17, 18] as const
export const CODE_FONT_SIZES = [11, 12, 13, 14, 15, 16] as const

export const CODE_FONTS = [
  {
    id: 'jetbrains',
    label: 'JetBrains Mono',
    stack:
      "'JetBrains Mono Variable', ui-monospace, 'SF Mono', Menlo, monospace",
  },
  {
    id: 'system',
    label: 'System',
    stack: "ui-monospace, 'SF Mono', Menlo, monospace",
  },
  {
    id: 'geist',
    label: 'Geist Mono',
    stack: "'Geist Mono Variable', ui-monospace, 'SF Mono', Menlo, monospace",
  },
] as const

export type CodeFontId = (typeof CODE_FONTS)[number]['id']

export const DEFAULT_UI_FONT_SIZE = 16
export const DEFAULT_CODE_FONT_SIZE = 12
export const DEFAULT_CODE_FONT: CodeFontId = 'jetbrains'

const THEME_KEY = 'cormux-theme'
const WORDMARK_KEY = 'cormux-wordmark'
const UI_FONT_KEY = 'cormux-ui-font-size'
const CODE_FONT_KEY = 'cormux-code-font-size'
const CODE_FONT_FAMILY_KEY = 'cormux-code-font'

export function codeFontStack(id: string | null) {
  return CODE_FONTS.find((font) => font.id === id)?.stack
}

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
  theme = $state<ThemeId>(DEFAULT_THEME)
  /** The big "cormux" drawn faintly behind the glass. */
  showWordmark = $state(true)
  uiFontSize = $state(DEFAULT_UI_FONT_SIZE)
  codeFontSize = $state(DEFAULT_CODE_FONT_SIZE)
  codeFont = $state<CodeFontId>(DEFAULT_CODE_FONT)

  constructor() {
    const theme = read(THEME_KEY)
    if (THEMES.some((row) => row.id === theme)) this.theme = theme as ThemeId
    this.showWordmark = read(WORDMARK_KEY) !== '0'
    this.uiFontSize = readSize(UI_FONT_KEY, UI_FONT_SIZES, DEFAULT_UI_FONT_SIZE)
    this.codeFontSize = readSize(
      CODE_FONT_KEY,
      CODE_FONT_SIZES,
      DEFAULT_CODE_FONT_SIZE,
    )
    const codeFont = read(CODE_FONT_FAMILY_KEY)
    if (codeFontStack(codeFont)) this.codeFont = codeFont as CodeFontId
    this.apply()
  }

  setTheme(next: ThemeId) {
    if (!THEMES.some((row) => row.id === next)) return
    this.theme = next
    write(THEME_KEY, next)
    this.apply()
  }

  setShowWordmark(next: boolean) {
    this.showWordmark = next
    write(WORDMARK_KEY, next ? '1' : '0')
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

  setCodeFont(next: CodeFontId) {
    if (!codeFontStack(next)) return
    this.codeFont = next
    write(CODE_FONT_FAMILY_KEY, next)
    this.apply()
  }

  private apply() {
    if (typeof document === 'undefined') return
    const root = document.documentElement
    if (this.theme === 'neutral') delete root.dataset.palette
    else root.dataset.palette = this.theme
    // Tailwind sizes are rem, so the root size scales text across the UI.
    root.style.fontSize =
      this.uiFontSize === DEFAULT_UI_FONT_SIZE ? '' : `${this.uiFontSize}px`
    root.style.setProperty('--code-font-size', `${this.codeFontSize}px`)
    const stack = codeFontStack(this.codeFont)
    if (stack) root.style.setProperty('--code-font-family', stack)
  }
}

export const appearance = new AppearanceState()
