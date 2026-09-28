import { describe, expect, it } from 'vitest'
import { eventCreatedAtMs, formatThreadTime } from './thread-time'

describe('eventCreatedAtMs', () => {
  it('reads sqlite utc datetime to the minute', () => {
    const ms = eventCreatedAtMs('2026-09-28 03:03:44')
    expect(formatThreadTime(ms)).toMatch(/\d{1,2}:03/)
  })
})
