import { describe, expect, it } from 'vitest'
import {
  DARK_FOREGROUND,
  LIGHT_FOREGROUND,
  contrastRatio,
  normalizeHex,
  parseHex,
  readableForeground,
  relativeLuminance,
} from './color'

describe('normalizeHex', () => {
  it('accepts short and long forms with or without #', () => {
    expect(normalizeHex('#ABC')).toBe('#aabbcc')
    expect(normalizeHex('abc')).toBe('#aabbcc')
    expect(normalizeHex(' #1170E0 ')).toBe('#1170e0')
    expect(normalizeHex('1170e0')).toBe('#1170e0')
  })

  it('rejects anything else', () => {
    for (const bad of ['', '#12', '#12345', '#ggg', 'blue', '#1170e0ff']) {
      expect(normalizeHex(bad)).toBeNull()
    }
    expect(normalizeHex(null)).toBeNull()
    expect(normalizeHex(undefined)).toBeNull()
  })
})

describe('parseHex', () => {
  it('splits channels', () => {
    expect(parseHex('#1170e0')).toEqual({ r: 0x11, g: 0x70, b: 0xe0 })
    expect(parseHex('#fff')).toEqual({ r: 255, g: 255, b: 255 })
    expect(parseHex('nope')).toBeNull()
  })
})

describe('relativeLuminance', () => {
  it('spans black to white', () => {
    expect(relativeLuminance({ r: 0, g: 0, b: 0 })).toBe(0)
    expect(relativeLuminance({ r: 255, g: 255, b: 255 })).toBeCloseTo(1)
    expect(relativeLuminance(parseHex('#808080')!)).toBeCloseTo(0.2159, 3)
  })

  it('weights green above red above blue', () => {
    const red = relativeLuminance({ r: 255, g: 0, b: 0 })
    const green = relativeLuminance({ r: 0, g: 255, b: 0 })
    const blue = relativeLuminance({ r: 0, g: 0, b: 255 })
    expect(green).toBeGreaterThan(red)
    expect(red).toBeGreaterThan(blue)
  })
})

describe('contrastRatio', () => {
  it('is 21 for black on white and symmetric', () => {
    expect(contrastRatio(1, 0)).toBeCloseTo(21)
    expect(contrastRatio(0, 1)).toBeCloseTo(21)
    expect(contrastRatio(0.3, 0.3)).toBe(1)
  })
})

describe('readableForeground', () => {
  it('puts white text on dark or saturated colours', () => {
    for (const hex of ['#000000', '#1170e0', '#794ddf', '#d63053']) {
      expect(readableForeground(hex)).toBe(LIGHT_FOREGROUND)
    }
  })

  it('puts dark text on light colours', () => {
    for (const hex of ['#ffffff', '#eab532', '#ffff00', '#7fffd4']) {
      expect(readableForeground(hex)).toBe(DARK_FOREGROUND)
    }
  })

  it('falls back to white for invalid input', () => {
    expect(readableForeground('nope')).toBe(LIGHT_FOREGROUND)
    expect(readableForeground(null)).toBe(LIGHT_FOREGROUND)
  })
})
