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
    expect(
      threadActivity('provisioning', {
        activity: 'Running worktree setup…',
        provStep: 1,
      }),
    ).toBe('Setting up the worktree')
    expect(
      threadActivity('provisioning', {
        activity: 'Starting agent…',
        provStep: 2,
      }),
    ).toBe('Starting agent…')
  })
})

describe('workspaceActivity', () => {
  it("prefers the core's own words", () => {
    expect(
      workspaceActivity({
        status: 'provisioning',
        kind: 'lead',
        activity: 'Installing dependencies…',
      }),
    ).toBe('Installing dependencies…')
  })

  it('falls back to the lifecycle, by kind while setting up', () => {
    expect(
      workspaceActivity({
        status: 'provisioning',
        kind: 'review',
        activity: '',
      }),
    ).toBe('Checking out the PR…')
    expect(
      workspaceActivity({
        status: 'provisioning',
        kind: 'lead',
        activity: ' ',
      }),
    ).toBe('Running worktree setup…')
    expect(
      workspaceActivity({
        status: 'provisioningFailed',
        kind: 'lead',
        activity: '',
      }),
    ).toBe('Setup failed')
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
