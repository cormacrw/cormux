/**
 * Per-machine look and feel. Kept in localStorage (like the light/dark mode, which
 * mode-watcher owns) so index.html can apply it before first paint; the values and keys
 * both read are in appearance-tokens.ts.
 */
import { readPref, writePref } from '$lib/primitives/local-pref'
import {
  APPEARANCE_KEYS,
  CODE_FONT_SIZES,
  codeFontStack,
  DEFAULT_CODE_FONT,
  DEFAULT_CODE_FONT_SIZE,
  DEFAULT_THEME,
  DEFAULT_UI_FONT_SIZE,
  THEMES,
  UI_FONT_SIZES,
  type CodeFontId,
  type ThemeId,
} from './appearance-tokens'

const {
  theme: THEME_KEY,
  wordmark: WORDMARK_KEY,
  uiFontSize: UI_FONT_KEY,
  codeFontSize: CODE_FONT_KEY,
  codeFont: CODE_FONT_FAMILY_KEY,
} = APPEARANCE_KEYS

function readSize(key: string, allowed: readonly number[], fallback: number) {
  const value = Number(readPref(key))
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
    const theme = readPref(THEME_KEY)
    if (THEMES.some((row) => row.id === theme)) this.theme = theme as ThemeId
    this.showWordmark = readPref(WORDMARK_KEY) !== '0'
    this.uiFontSize = readSize(UI_FONT_KEY, UI_FONT_SIZES, DEFAULT_UI_FONT_SIZE)
    this.codeFontSize = readSize(
      CODE_FONT_KEY,
      CODE_FONT_SIZES,
      DEFAULT_CODE_FONT_SIZE,
    )
    const codeFont = readPref(CODE_FONT_FAMILY_KEY)
    if (codeFontStack(codeFont)) this.codeFont = codeFont as CodeFontId
    this.apply()
  }

  setTheme(next: ThemeId) {
    if (!THEMES.some((row) => row.id === next)) return
    this.theme = next
    writePref(THEME_KEY, next)
    this.apply()
  }

  setShowWordmark(next: boolean) {
    this.showWordmark = next
    writePref(WORDMARK_KEY, next ? '1' : '0')
  }

  setUiFontSize(next: number) {
    if (!UI_FONT_SIZES.includes(next as never)) return
    this.uiFontSize = next
    writePref(UI_FONT_KEY, String(next))
    this.apply()
  }

  setCodeFontSize(next: number) {
    if (!CODE_FONT_SIZES.includes(next as never)) return
    this.codeFontSize = next
    writePref(CODE_FONT_KEY, String(next))
    this.apply()
  }

  setCodeFont(next: CodeFontId) {
    if (!codeFontStack(next)) return
    this.codeFont = next
    writePref(CODE_FONT_FAMILY_KEY, next)
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
