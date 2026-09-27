import { describe, expect, it } from 'vitest'
import { hasVersionGap } from './version'

describe('hasVersionGap', () => {
  it('is false for the next sequential version', () => {
    expect(hasVersionGap(3, 4)).toBe(false)
  })

  it('is true when at least one version was skipped', () => {
    expect(hasVersionGap(3, 5)).toBe(true)
  })
})
