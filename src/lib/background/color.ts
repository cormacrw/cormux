/**
 * Small colour helpers for the accent picker and the background scenes. Pure
 * and dependency free, so the appearance state and index.html's pre-paint
 * script (which mirrors `readableForeground`) can share the same rules.
 */

export type Rgb = { r: number; g: number; b: number }

const HEX = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i

/** `#abc`, `abc`, `#aabbcc` or `aabbcc` to lower-case `#aabbcc`; null when invalid. */
export function normalizeHex(value: string | null | undefined): string | null {
  const match = HEX.exec((value ?? '').trim())
  if (!match) return null
  let digits = (match[1] ?? '').toLowerCase()
  if (digits.length === 3) {
    digits = digits
      .split('')
      .map((digit) => digit + digit)
      .join('')
  }
  return `#${digits}`
}

/** Channels 0 to 255, or null when the hex is invalid. */
export function parseHex(value: string | null | undefined): Rgb | null {
  const hex = normalizeHex(value)
  if (!hex) return null
  const n = Number.parseInt(hex.slice(1), 16)
  return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255 }
}

function linear(channel: number) {
  const c = channel / 255
  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
}

/** WCAG relative luminance, 0 (black) to 1 (white). */
export function relativeLuminance({ r, g, b }: Rgb): number {
  return 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}

/** WCAG contrast ratio between two luminances, 1 to 21. */
export function contrastRatio(a: number, b: number): number {
  const [hi, lo] = a > b ? [a, b] : [b, a]
  return (hi + 0.05) / (lo + 0.05)
}

export const LIGHT_FOREGROUND = '#ffffff'
export const DARK_FOREGROUND = '#17171c'
const DARK_FOREGROUND_LUMINANCE = relativeLuminance(parseHex(DARK_FOREGROUND)!)

/**
 * Text colour for content on top of `hex`: white unless dark text reads
 * better. Invalid input gets white, matching the built-in swatches.
 */
export function readableForeground(hex: string | null | undefined): string {
  const rgb = parseHex(hex)
  if (!rgb) return LIGHT_FOREGROUND
  const lum = relativeLuminance(rgb)
  return contrastRatio(lum, DARK_FOREGROUND_LUMINANCE) > contrastRatio(lum, 1)
    ? DARK_FOREGROUND
    : LIGHT_FOREGROUND
}
