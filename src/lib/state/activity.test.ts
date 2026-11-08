import { describe, expect, it } from 'vitest'
import {
  runningAgentCount,
  threadActivity,
  workspaceActivity,
  workspaceStatusLine,
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

  // Bug: the sidebar and repo page said "Running"/"Starting" where the rest of the app said
  // "Working", and an idle workspace went on reading "Reading repo…" after its first turn.
  it('says paused, and the first read only while the agent works', () => {
    const lead = {
      kind: 'lead' as const,
      activity: { kind: 'agentStarted' as const },
    }
    expect(workspaceActivity({ ...lead, status: 'paused' })).toBe('Paused')
    expect(workspaceActivity({ ...lead, status: 'idle' })).toBe('Idle')
    expect(
      workspaceActivity({
        ...lead,
        status: 'running',
        activity: { kind: 'none' },
      }),
    ).toBe('Working')
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

describe('workspaceStatusLine', () => {
  it('puts a failed setup before approvals, and keeps it on show', () => {
    const failed = {
      lifecycle: 'provisioningFailed' as const,
      pendingApprovals: 2,
      activityText: workspaceActivity({
        status: 'provisioningFailed',
        kind: 'lead',
        activity: { kind: 'settingUp' },
      }),
    }
    expect(workspaceStatusLine(failed)).toEqual({
      text: 'Setup failed',
      failed: true,
    })
    expect(
      workspaceStatusLine({
        ...failed,
        lifecycle: 'running',
        activityText: 'Working',
      }),
    ).toEqual({ text: '2 approvals waiting', failed: false })
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
