import { describe, expect, it } from 'vitest'
import { formatThreadTime } from './thread-time'

describe('formatThreadTime', () => {
  it('shows the local clock time to the minute, without seconds', () => {
    // Local time, so the minutes hold in a zone offset by half an hour.
    const label = formatThreadTime(new Date(2026, 8, 28, 3, 7, 44).getTime())
    expect(label).toMatch(/\b0?3:07\b/)
    expect(label).not.toContain('44')
  })

  it('is empty without a timestamp', () => {
    expect(formatThreadTime(0)).toBe('')
  })
})
