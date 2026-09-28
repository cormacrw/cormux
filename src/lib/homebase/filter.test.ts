import { describe, expect, it } from 'vitest'
import {
  homebaseFilterCounts,
  isIdleLifecycle,
  matchesHomebaseFilter,
} from './filter'

describe('homebase filter', () => {
  it('treats ready and idle as idle filter bucket', () => {
    expect(isIdleLifecycle('idle')).toBe(true)
    expect(isIdleLifecycle('ready')).toBe(true)
    expect(isIdleLifecycle('running')).toBe(false)
  })

  it('running filter includes provisioning', () => {
    expect(matchesHomebaseFilter('provisioning', 'running')).toBe(true)
    expect(matchesHomebaseFilter('idle', 'running')).toBe(false)
  })

  it('counts from the full list', () => {
    expect(
      homebaseFilterCounts(['running', 'idle', 'ready', 'provisioning']),
    ).toEqual({ all: 4, running: 2, idle: 2 })
  })
})
