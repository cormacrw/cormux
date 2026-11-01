import { describe, expect, it } from 'vitest'
import {
  formatMemoryGb,
  plural,
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

describe('formatMemoryGb', () => {
  it('formats sub-10 GB with one decimal', () => {
    expect(formatMemoryGb(1.4 * 1024 ** 3)).toBe('1.4 GB')
  })
})

describe('plural', () => {
  it('uses singular and plural labels', () => {
    expect(plural(1, 'agent')).toBe('1 agent')
    expect(plural(3, 'agent')).toBe('3 agents')
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
