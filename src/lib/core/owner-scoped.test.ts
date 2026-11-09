import { describe, expect, it } from 'vitest'
import { latestRequests, startsOver } from './owner-scoped'

describe('latestRequests', () => {
  it('lets only the latest request for a key land', () => {
    const request = latestRequests()
    const first = request('ws-1')
    const second = request('ws-1')
    expect(first()).toBe(false)
    expect(second()).toBe(true)
  })

  it('keeps keys apart', () => {
    const request = latestRequests()
    const a = request('ws-1')
    request('ws-2')
    expect(a()).toBe(true)
  })
})

describe('startsOver', () => {
  it('is the initial value, whatever the owner', () => {
    expect(startsOver('th-1', 0)).toBe(0)
    expect(startsOver(null, null)).toBe(null)
  })
})
