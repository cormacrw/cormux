import { describe, expect, it } from 'vitest'
import {
  formatMemoryGb,
  isActiveThread,
  memoryBarScale,
  plural,
  statusDotVariantForThread,
  statusDotVariantForWorkspace,
  threadActivityLine,
  workspaceStatusWord,
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

describe('thread sidebar helpers', () => {
  it('shows Paused on the activity line', () => {
    expect(
      threadActivityLine({
        status: 'running',
        paused: true,
        activity: 'Editing files',
      }),
    ).toBe('Paused')
  })

  it('counts active threads', () => {
    expect(
      isActiveThread({ status: 'running', paused: false, activity: '' }),
    ).toBe(true)
    expect(
      isActiveThread({ status: 'idle', paused: false, activity: '' }),
    ).toBe(false)
  })

  it('maps thread dots', () => {
    expect(
      statusDotVariantForThread({
        status: 'running',
        paused: false,
        activity: '',
      }),
    ).toBe('running')
  })
})

describe('formatMemoryGb', () => {
  it('formats sub-10 GB with one decimal', () => {
    expect(formatMemoryGb(1.4 * 1024 ** 3)).toBe('1.4 GB')
  })
})

describe('memoryBarScale', () => {
  it('caps at 1', () => {
    expect(memoryBarScale(8 * 1024 ** 3)).toBe(1)
  })
})

describe('plural', () => {
  it('uses singular and plural labels', () => {
    expect(plural(1, 'agent')).toBe('1 agent')
    expect(plural(3, 'agent')).toBe('3 agents')
  })
})
