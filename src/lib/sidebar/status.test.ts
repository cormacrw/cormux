import { describe, expect, it } from 'vitest'
import {
  statusDotVariantForWorkspace,
  workspaceStatusWord,
  statusDotVariantForAttention,
} from './status'

describe('statusDotVariantForWorkspace', () => {
  it('maps provisioning lifecycles', () => {
    expect(
      statusDotVariantForWorkspace({
        lifecycle: 'provisioning',
        activityText: 'Idle',
      }),
    ).toBe('provisioning')
  })

  it('maps running and paused', () => {
    expect(
      statusDotVariantForWorkspace({
        lifecycle: 'running',
        activityText: '',
      }),
    ).toBe('running')
    expect(
      statusDotVariantForWorkspace({ lifecycle: 'paused', activityText: '' }),
    ).toBe('paused')
  })
})

describe('workspaceStatusWord', () => {
  it('uses activity text when idle', () => {
    expect(
      workspaceStatusWord({
        lifecycle: 'idle',
        activityText: 'Plan ready',
      }),
    ).toBe('Plan ready')
  })

  it('says a paused workspace is paused, whatever it was last doing', () => {
    expect(
      workspaceStatusWord({
        lifecycle: 'paused',
        activityText: 'Reading repo…',
      }),
    ).toBe('Paused')
  })

  it('uses Starting and Running labels', () => {
    expect(
      workspaceStatusWord({
        lifecycle: 'provisioning',
        activityText: '',
      }),
    ).toBe('Starting')
    expect(
      workspaceStatusWord({
        lifecycle: 'running',
        activityText: '',
      }),
    ).toBe('Running')
  })
})

describe('statusDotVariantForAttention', () => {
  it('flashes only while the agent is on hold for an answer', () => {
    expect(statusDotVariantForAttention(1)).toBe('waiting')
    expect(statusDotVariantForAttention(0)).toBe('ready')
  })
})
