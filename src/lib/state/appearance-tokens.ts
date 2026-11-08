// Appearance's values, in one place for the app (appearance.svelte.ts) and the pre-paint
// script in index.html, which gets `bootAppearance()` injected at build time (vite.config.ts).
// Plain data with no imports, so the Vite config can load it.

/** Where each choice is kept in localStorage. */
export const APPEARANCE_KEYS = {
  theme: 'cormux-theme',
  wordmark: 'cormux-wordmark',
  uiFontSize: 'cormux-ui-font-size',
  codeFontSize: 'cormux-code-font-size',
  codeFont: 'cormux-code-font',
} as const

/**
 * Colour themes. Neutral is grey glass with a blue primary; the others tint
 * the window, the glass and the primary with one bold hue. The hues live in
 * app.css (`data-palette` on <html>); `hue` and `strength` (--tint-s,
 * default 1) here draw the picker's previews and the first paint, so keep
 * them in sync with app.css.
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

export function codeFontStack(id: string | null) {
  return CODE_FONTS.find((font) => font.id === id)?.stack
}

/** What index.html's script needs to apply the saved choices before app.css loads. */
export function bootAppearance() {
  return {
    keys: APPEARANCE_KEYS,
    defaultTheme: DEFAULT_THEME,
    /** Each tinted theme's `[hue, strength]`; Neutral has none. */
    palettes: Object.fromEntries(
      THEMES.flatMap((theme) =>
        theme.hue == null
          ? []
          : [[theme.id, [theme.hue, 'strength' in theme ? theme.strength : 1]]],
      ),
    ),
    uiFontSizes: UI_FONT_SIZES,
    codeFontSizes: CODE_FONT_SIZES,
    codeFonts: Object.fromEntries(
      CODE_FONTS.map((font) => [font.id, font.stack]),
    ),
  }
}
