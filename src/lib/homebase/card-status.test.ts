import { describe, expect, it } from 'vitest'
import { workspaceCardMetaText } from './card-status'

const baseWorkspace = {
  id: 'ws',
  name: 'Auth',
  lifecycle: 'idle' as const,
  paused: false,
  activityText: 'Idle',
  pendingApprovals: 0,
  cardStatus: 'idle' as const,
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
})
