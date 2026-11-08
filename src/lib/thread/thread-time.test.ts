import { describe, expect, it } from 'vitest'
import { formatThreadTime } from './thread-time'

describe('formatThreadTime', () => {
  it('shows the clock time to the minute', () => {
    expect(formatThreadTime(Date.UTC(2026, 8, 28, 3, 3, 44))).toMatch(
      /\d{1,2}:03/,
    )
  })

  it('is empty without a timestamp', () => {
    expect(formatThreadTime(0)).toBe('')
  })
})
