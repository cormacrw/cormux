import { describe, expect, it } from 'vitest'
import { NO_USAGE } from '$lib/usage/usage'
import { workspaceCardBadgeKind, workspaceCardMetaText } from './card-status'

const baseWorkspace = {
  id: 'ws',
  name: 'Auth',
  branch: 'feat/auth',
  lifecycle: 'idle' as const,
  paused: false,
  activityText: 'Idle',
  pendingApprovals: 0,
  retired: false,
  usage: NO_USAGE,
  cardStatus: 'idle' as const,
  createdAtMs: null,
  summary: null,
  summaryAtMs: null,
  summarySource: 'Haiku 4.5',
  kind: null,
  prNumber: null,
  prHtmlUrl: null,
  modifiedFiles: 0,
  provStep: 0,
  setupFailedCommand: null,
  setupFailedExitCode: null,
}

describe('workspaceCardMetaText', () => {
  it('returns Needs Attention when approvals pending', () => {
    const text = workspaceCardMetaText(baseWorkspace, [
      {
        id: 't',
        workspaceId: 'ws',
        role: 'Lead',
        engine: 'claude',
        status: 'running',
        paused: false,
        activity: 'Working',
        pendingApprovals: 1,
        retired: false,
        usage: NO_USAGE,
      },
    ])
    expect(text).toBe('Needs Attention')
  })

  it('returns agent working count', () => {
    const text = workspaceCardMetaText(baseWorkspace, [
      {
        id: 't1',
        workspaceId: 'ws',
        role: 'Lead',
        engine: 'claude',
        status: 'running',
        paused: false,
        activity: 'Working',
        pendingApprovals: 0,
        retired: false,
        usage: NO_USAGE,
      },
      {
        id: 't2',
        workspaceId: 'ws',
        role: 'Reviewer',
        engine: 'claude',
        status: 'running',
        paused: false,
        activity: 'Working',
        pendingApprovals: 0,
        retired: false,
        usage: NO_USAGE,
      },
    ])
    expect(text).toBe('2 agents working')
  })

  it('returns Idle when nothing active', () => {
    expect(workspaceCardMetaText(baseWorkspace, [])).toBe('Idle')
  })

  it('prioritizes needs attention over working', () => {
    expect(
      workspaceCardBadgeKind(baseWorkspace, [
        {
          id: 't',
          workspaceId: 'ws',
          role: 'Lead',
          engine: 'claude',
          status: 'running',
          paused: false,
          activity: 'Working',
          pendingApprovals: 1,
          retired: false,
          usage: NO_USAGE,
        },
      ]),
    ).toBe('needsAttention')
  })
})
