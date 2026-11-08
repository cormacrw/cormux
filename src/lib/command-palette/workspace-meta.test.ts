import { describe, expect, it } from 'vitest'
import { threadFixture } from '$lib/state/fixtures'
import { workspaceCardMetaText } from './workspace-meta'

describe('workspaceCardMetaText', () => {
  it('says the workspace needs you when the core says so, even if agents work', () => {
    expect(
      workspaceCardMetaText(true, [
        threadFixture({ status: 'running', pendingApprovals: 1 }),
        threadFixture({ id: 'th-2', status: 'running' }),
      ]),
    ).toBe('Needs Attention')
  })

  it('counts working agents', () => {
    expect(
      workspaceCardMetaText(false, [
        threadFixture({ status: 'running' }),
        threadFixture({ id: 'th-2', status: 'running' }),
        threadFixture({ id: 'th-3', status: 'paused' }),
      ]),
    ).toBe('2 agents working')
  })
})
