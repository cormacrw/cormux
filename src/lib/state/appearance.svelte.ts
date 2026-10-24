/**
 * Per-machine look and feel. Kept in localStorage (like the theme, which
 * mode-watcher owns) so index.html can apply it before first paint; keep the
 * storage keys in sync with the script there.
 */

import { normalizeHex, readableForeground } from '$lib/background/color'
import {
  DEFAULT_BACKGROUND,
  isBackgroundId,
  type BackgroundId,
} from '$lib/background/registry'

/**
 * Built-in swatches. `hex` is the sRGB equivalent of the `--swatch-*` oklch
 * value in app.css; the background scenes read it. Graphite follows the
 * theme in CSS, so its hex is a mid grey that suits both.
 */
export const ACCENTS = [
  { id: 'neutral', label: 'Graphite', hex: '#8a8f98' },
  { id: 'blue', label: 'Blue', hex: '#1170e0' },
  { id: 'violet', label: 'Violet', hex: '#794ddf' },
  { id: 'green', label: 'Green', hex: '#008b4e' },
  { id: 'gold', label: 'Gold', hex: '#eab532' },
  { id: 'orange', label: 'Orange', hex: '#d65500' },
  { id: 'rose', label: 'Rose', hex: '#d63053' },
] as const

export type SwatchId = (typeof ACCENTS)[number]['id']
/** A swatch, or `custom` for the hex in `appearance.customAccent`. */
export type AccentId = SwatchId | 'custom'

export const DEFAULT_ACCENT: SwatchId = 'blue'
export const DEFAULT_CUSTOM_ACCENT = '#1170e0'

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

const ACCENT_KEY = 'cormux-accent'
const CUSTOM_ACCENT_KEY = 'cormux-accent-custom'
const BACKGROUND_KEY = 'cormux-background'
const ANIMATE_BACKGROUND_KEY = 'cormux-background-animate'
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
  accent = $state<AccentId>(DEFAULT_ACCENT)
  /** Lower-case `#rrggbb`, used when `accent` is `custom`. */
  customAccent = $state(DEFAULT_CUSTOM_ACCENT)
  background = $state<BackgroundId>(DEFAULT_BACKGROUND)
  animateBackground = $state(true)
  uiFontSize = $state(DEFAULT_UI_FONT_SIZE)
  codeFontSize = $state(DEFAULT_CODE_FONT_SIZE)
  codeFont = $state<CodeFontId>(DEFAULT_CODE_FONT)

  constructor() {
    const custom = normalizeHex(read(CUSTOM_ACCENT_KEY))
    if (custom) this.customAccent = custom
    const accent = read(ACCENT_KEY)
    if (accent === 'custom' || ACCENTS.some((row) => row.id === accent)) {
      this.accent = accent as AccentId
    }
    const background = read(BACKGROUND_KEY)
    if (isBackgroundId(background)) this.background = background
    this.animateBackground = read(ANIMATE_BACKGROUND_KEY) !== '0'
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

  /** The current accent as `#rrggbb`, for the background scenes. */
  get accentHex(): string {
    if (this.accent === 'custom') return this.customAccent
    return ACCENTS.find((row) => row.id === this.accent)?.hex ?? ACCENTS[1].hex
  }

  setAccent(next: AccentId) {
    if (next !== 'custom' && !ACCENTS.some((row) => row.id === next)) return
    this.accent = next
    write(ACCENT_KEY, next)
    this.apply()
  }

  /** Switch to a custom accent; ignores anything that isn't a hex colour. */
  setCustomAccent(hex: string) {
    const next = normalizeHex(hex)
    if (!next) return
    this.customAccent = next
    write(CUSTOM_ACCENT_KEY, next)
    this.setAccent('custom')
  }

  setBackground(next: BackgroundId) {
    if (!isBackgroundId(next)) return
    this.background = next
    write(BACKGROUND_KEY, next)
    this.apply()
  }

  setAnimateBackground(next: boolean) {
    this.animateBackground = next
    write(ANIMATE_BACKGROUND_KEY, next ? '1' : '0')
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
    if (this.accent === 'neutral') delete root.dataset.accent
    else root.dataset.accent = this.accent
    // Swatches come from app.css; a custom colour is set inline.
    if (this.accent === 'custom') {
      root.style.setProperty('--primary', this.customAccent)
      root.style.setProperty(
        '--primary-foreground',
        readableForeground(this.customAccent),
      )
    } else {
      root.style.removeProperty('--primary')
      root.style.removeProperty('--primary-foreground')
    }
    root.dataset.background = this.background
    // Tailwind sizes are rem, so the root size scales text across the UI.
    root.style.fontSize =
      this.uiFontSize === DEFAULT_UI_FONT_SIZE ? '' : `${this.uiFontSize}px`
    root.style.setProperty('--code-font-size', `${this.codeFontSize}px`)
    const stack = codeFontStack(this.codeFont)
    if (stack) root.style.setProperty('--code-font-family', stack)
  }
}

export const appearance = new AppearanceState()
