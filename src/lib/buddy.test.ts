import { describe, expect, it } from 'vitest'
import { buddyColor, buddyMoodForThread } from './buddy'

const thread = { status: 'idle', paused: false, pendingApprovals: 0 }

describe('buddyMoodForThread', () => {
  it('lets the face follow the thread', () => {
    expect(buddyMoodForThread({ ...thread, pendingApprovals: 1 })).toBe(
      'needsYou',
    )
    expect(buddyMoodForThread({ ...thread, status: 'provisioning' })).toBe(
      'starting',
    )
    expect(buddyMoodForThread({ ...thread, status: 'running' })).toBe('running')
    expect(buddyMoodForThread({ ...thread, paused: true })).toBe('paused')
    expect(buddyMoodForThread(thread)).toBe('idle')
  })
})

describe('buddyColor', () => {
  it('stays the same for a thread and can differ between threads', () => {
    expect(buddyColor('th-lead')).toBe(buddyColor('th-lead'))
    const colors = ['th-lead', 'th-a', 'th-b', 'ws-1', 'scratch-1'].map(
      buddyColor,
    )
    expect(new Set(colors).size).toBeGreaterThan(1)
  })
})
