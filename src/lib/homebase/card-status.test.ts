import { describe, expect, it } from 'vitest'
import { workspaceCardBadgeKind, workspaceCardMetaText } from './card-status'

const baseWorkspace = {
  id: 'ws',
  name: 'Auth',
  branch: 'feat/auth',
  lifecycle: 'idle' as const,
  paused: false,
  activityText: 'Idle',
  pendingApprovals: 0,
  cardStatus: 'idle' as const,
  createdAtMs: null,
  summary: null,
  summaryAtMs: null,
  summarySource: 'Haiku 4.5',
  kind: null,
  prNumber: null,
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
        },
      ]),
    ).toBe('needsAttention')
  })
})
