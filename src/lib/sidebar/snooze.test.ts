import { describe, expect, it } from 'vitest'
import { SNOOZE_OPTIONS, formatSnoozeUntil, splitSnoozed } from './snooze'

const option = (id: string) => SNOOZE_OPTIONS.find((o) => o.id === id)!
// Wednesday 12 November 2025, 14:30 local time.
const wednesday = new Date(2025, 10, 12, 14, 30).getTime()

describe('snooze options', () => {
  it('offers four presets', () => {
    expect(SNOOZE_OPTIONS.map((o) => o.label)).toEqual([
      '1 hour',
      '3 hours',
      'Tomorrow',
      'Next week',
    ])
  })

  it('counts hours from now', () => {
    expect(option('hour').until(wednesday)).toBe(wednesday + 3_600_000)
    expect(option('three-hours').until(wednesday)).toBe(
      wednesday + 3 * 3_600_000,
    )
  })

  it('wakes tomorrow at 9', () => {
    expect(option('tomorrow').until(wednesday)).toBe(
      new Date(2025, 10, 13, 9).getTime(),
    )
  })

  it('wakes next Monday at 9, a full week out from a Monday', () => {
    expect(option('next-week').until(wednesday)).toBe(
      new Date(2025, 10, 17, 9).getTime(),
    )
    const monday = new Date(2025, 10, 17, 8).getTime()
    expect(option('next-week').until(monday)).toBe(
      new Date(2025, 10, 24, 9).getTime(),
    )
    const sunday = new Date(2025, 10, 16, 20).getTime()
    expect(option('next-week').until(sunday)).toBe(
      new Date(2025, 10, 17, 9).getTime(),
    )
  })
})

describe('splitSnoozed', () => {
  it('moves threads whose snooze has not run out', () => {
    const items = [{ id: 'a' }, { id: 'b' }, { id: 'c' }]
    expect(splitSnoozed(items, { a: 50, b: 200 }, 100)).toEqual({
      awake: [{ id: 'a' }, { id: 'c' }],
      snoozed: [{ id: 'b' }],
    })
  })
})

describe('formatSnoozeUntil', () => {
  it('names the day only when it is not today', () => {
    const later = new Date(2025, 10, 12, 17).getTime()
    expect(formatSnoozeUntil(later, wednesday)).not.toMatch(/Tomorrow/)
    expect(
      formatSnoozeUntil(new Date(2025, 10, 13, 9).getTime(), wednesday),
    ).toMatch(/^Tomorrow /)
  })

  it('gives just the day when short', () => {
    const friday = new Date(2025, 10, 14, 9)
    expect(
      formatSnoozeUntil(friday.getTime(), wednesday, { short: true }),
    ).toBe(friday.toLocaleDateString(undefined, { weekday: 'short' }))
  })
})
