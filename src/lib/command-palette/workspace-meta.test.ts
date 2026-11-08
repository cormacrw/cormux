import { describe, expect, it } from 'vitest'
import { threadFixture, workspaceFixture } from '$lib/state/fixtures'
import { workspaceCardMetaText } from './workspace-meta'

const running = workspaceFixture({ lifecycle: 'running' })

describe('workspaceCardMetaText', () => {
  it('says the workspace needs you while an approval waits, even if agents work', () => {
    expect(
      workspaceCardMetaText({ ...running, pendingApprovals: 1 }, [
        threadFixture({ status: 'running', pendingApprovals: 1 }),
        threadFixture({ id: 'th-2', status: 'running' }),
      ]),
    ).toBe('Needs Attention')
  })

  it('counts working agents', () => {
    expect(
      workspaceCardMetaText(running, [
        threadFixture({ status: 'running' }),
        threadFixture({ id: 'th-2', status: 'running' }),
        threadFixture({ id: 'th-3', status: 'paused' }),
      ]),
    ).toBe('2 agents working')
  })

  it('needs you when nothing is working, like the sidebar', () => {
    expect(workspaceCardMetaText(workspaceFixture(), [threadFixture()])).toBe(
      'Needs Attention',
    )
    expect(
      workspaceCardMetaText({ ...running, paused: true }, [
        threadFixture({ status: 'paused' }),
      ]),
    ).toBe('Needs Attention')
  })

  it('is working as soon as its thread is, before the workspace catches up', () => {
    expect(
      workspaceCardMetaText(workspaceFixture(), [
        threadFixture({ status: 'running' }),
      ]),
    ).toBe('1 agent working')
  })
})
