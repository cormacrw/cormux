import { describe, expect, it } from 'vitest'
import {
  runningAgentCount,
  threadActivity,
  workspaceActivity,
} from './activity'

describe('threadActivity', () => {
  it('names each status in a word', () => {
    expect(threadActivity('running')).toBe('Working')
    expect(threadActivity('paused')).toBe('Paused')
    expect(threadActivity('idle')).toBe('Idle')
  })

  it('follows setup while the workspace is being set up', () => {
    expect(threadActivity('provisioning', { kind: 'settingUp' })).toBe(
      'Setting up the worktree',
    )
    expect(threadActivity('provisioning', { kind: 'startingAgent' })).toBe(
      'Starting agent…',
    )
  })
})

describe('workspaceActivity', () => {
  it('words what the core says, by kind', () => {
    expect(
      workspaceActivity({
        status: 'provisioning',
        kind: 'review',
        activity: { kind: 'settingUp' },
      }),
    ).toBe('Checking out the PR…')
    expect(
      workspaceActivity({
        status: 'running',
        kind: 'lead',
        activity: { kind: 'agentStarted' },
      }),
    ).toBe('Reading repo…')
    expect(
      workspaceActivity({
        status: 'provisioningFailed',
        kind: 'lead',
        activity: { kind: 'setupFailed', command: 'pnpm i', exitCode: 1 },
      }),
    ).toBe('Setup failed')
  })

  it('falls back to the lifecycle, by kind while setting up', () => {
    expect(
      workspaceActivity({
        status: 'provisioning',
        kind: 'lead',
        activity: { kind: 'none' },
      }),
    ).toBe('Running worktree setup…')
    expect(
      workspaceActivity({
        status: 'ready',
        kind: 'lead',
        activity: { kind: 'none' },
      }),
    ).toBe('Ready')
  })
})

describe('runningAgentCount', () => {
  it('counts working and starting agents, not paused or idle ones', () => {
    expect(
      runningAgentCount([
        { status: 'running' },
        { status: 'paused' },
        { status: 'provisioning' },
        { status: 'idle' },
      ]),
    ).toBe(2)
  })
})
