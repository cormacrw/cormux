import { describe, expect, it } from 'vitest'
import { formatRelativeAge } from './relative-time'

const base = Date.parse('2026-01-01T12:00:00Z')

describe('formatRelativeAge', () => {
  it('returns just now under a minute', () => {
    expect(formatRelativeAge(base, base + 30_000)).toBe('just now')
  })

  it('returns minutes under an hour', () => {
    expect(formatRelativeAge(base, base + 12 * 60_000)).toBe('12m ago')
  })

  it('returns hours after an hour', () => {
    expect(formatRelativeAge(base, base + 3 * 60 * 60_000)).toBe('3h ago')
  })
})
