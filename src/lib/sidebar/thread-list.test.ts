import { describe, expect, it } from 'vitest'
import {
  nestThreadList,
  sortThreadList,
  splitThreadList,
  threadUrgency,
} from './thread-list'

describe('thread urgency', () => {
  const working = {
    lifecycle: 'running' as const,
    paused: false,
    pendingApprovals: 0,
  }

  it('is quiet only while working', () => {
    expect(threadUrgency(working, undefined)).toBe('quiet')
    expect(
      threadUrgency({ ...working, lifecycle: 'provisioning' }, undefined),
    ).toBe('quiet')
  })

  it('is working as soon as the thread is, before the workspace catches up', () => {
    expect(threadUrgency({ ...working, lifecycle: 'idle' }, 'running')).toBe(
      'quiet',
    )
  })

  it('needs attention whenever nothing is happening', () => {
    for (const lifecycle of ['idle', 'ready', 'provisioningFailed'] as const) {
      expect(threadUrgency({ ...working, lifecycle }, undefined)).toBe(
        'attention',
      )
    }
    expect(threadUrgency({ ...working, paused: true }, undefined)).toBe(
      'attention',
    )
  })

  it('needs attention on an approval, even mid-turn', () => {
    expect(threadUrgency({ ...working, pendingApprovals: 1 }, 'running')).toBe(
      'attention',
    )
  })
})

describe('thread order', () => {
  it('puts what needs you first, longest wait on top, then recent activity', () => {
    const sorted = sortThreadList([
      { id: 'old', urgency: 'quiet' as const, activityAtMs: 1 },
      { id: 'new', urgency: 'quiet' as const, activityAtMs: 9 },
      { id: 'late', urgency: 'attention' as const, activityAtMs: 8 },
      { id: 'early', urgency: 'attention' as const, activityAtMs: 3 },
    ])
    expect(sorted.map((row) => row.id)).toEqual(['early', 'late', 'new', 'old'])
  })
})

describe('thread sections', () => {
  const waiting = { id: 'a', urgency: 'attention' as const, workspace: {} }
  const working = { id: 'b', urgency: 'quiet' as const, workspace: {} }
  const leftover = { id: 'c', urgency: 'quiet' as const, workspace: null }
  const items = [waiting, working, leftover]

  it('moves working threads out of the list', () => {
    expect(splitThreadList(items, null)).toEqual({
      threads: [waiting, leftover],
      working: [working],
    })
  })

  it('leaves the thread you are on where it is', () => {
    expect(splitThreadList(items, 'b')).toEqual({
      threads: items,
      working: [],
    })
  })
})

describe('nestThreadList', () => {
  const item = (id: string, parentId: string | null = null) => ({
    id,
    parentId,
  })

  it('puts children right under their parent', () => {
    const rows = nestThreadList([
      item('kid', 'lead'),
      item('other'),
      item('lead'),
      item('kid-2', 'lead'),
    ])
    expect(rows.map((row) => [row.id, row.nested])).toEqual([
      ['other', false],
      ['lead', false],
      ['kid', true],
      ['kid-2', true],
    ])
  })

  it('keeps a child whose parent is not in the list', () => {
    const rows = nestThreadList([item('kid', 'gone'), item('other')])
    expect(rows.map((row) => [row.id, row.nested])).toEqual([
      ['kid', false],
      ['other', false],
    ])
  })
})
