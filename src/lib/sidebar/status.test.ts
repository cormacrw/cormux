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
        paused: false,
        activityText: 'Idle',
      }),
    ).toBe('provisioning')
  })

  it('maps running and paused', () => {
    expect(
      statusDotVariantForWorkspace({
        lifecycle: 'running',
        paused: false,
        activityText: '',
      }),
    ).toBe('running')
    expect(
      statusDotVariantForWorkspace({
        lifecycle: 'running',
        paused: true,
        activityText: '',
      }),
    ).toBe('paused')
  })
})

describe('workspaceStatusWord', () => {
  it('uses activity text when idle', () => {
    expect(
      workspaceStatusWord({
        lifecycle: 'idle',
        paused: false,
        activityText: 'Plan ready',
      }),
    ).toBe('Plan ready')
  })

  it('uses Starting and Running labels', () => {
    expect(
      workspaceStatusWord({
        lifecycle: 'provisioning',
        paused: false,
        activityText: '',
      }),
    ).toBe('Starting')
    expect(
      workspaceStatusWord({
        lifecycle: 'running',
        paused: false,
        activityText: '',
      }),
    ).toBe('Running')
  })
})

describe('statusDotVariantForAttention', () => {
  it('flashes only while the agent is on hold for an answer', () => {
    expect(statusDotVariantForAttention(1, 'running')).toBe('waiting')
    expect(statusDotVariantForAttention(0, 'waiting')).toBe('waiting')
    expect(statusDotVariantForAttention(0, 'idle')).toBe('ready')
    expect(statusDotVariantForAttention(0, undefined)).toBe('ready')
  })
})
