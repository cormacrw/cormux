import { describe, expect, it } from 'vitest'
import { statusDotFor } from './status'

describe('statusDotFor', () => {
  it('follows the lifecycle of a workspace that is getting on with it', () => {
    const dot = (lifecycle: Parameters<typeof statusDotFor>[0]['lifecycle']) =>
      statusDotFor({ lifecycle, pendingApprovals: 0 }, false)
    expect(dot('provisioning')).toBe('provisioning')
    expect(dot('running')).toBe('running')
    expect(dot('paused')).toBe('paused')
    expect(dot('idle')).toBe('idle')
  })

  // Bug: Ctrl+Tab showed a workspace waiting on an approval as running, where the sidebar flashed.
  it('says a workspace needs you first, flashing only while an agent waits on an answer', () => {
    expect(
      statusDotFor({ lifecycle: 'running', pendingApprovals: 1 }, true),
    ).toBe('waiting')
    expect(statusDotFor({ lifecycle: 'idle', pendingApprovals: 0 }, true)).toBe(
      'ready',
    )
  })
})
