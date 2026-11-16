import { describe, expect, it } from 'vitest'
import {
  nestThreadList,
  nextThreadAfterSend,
  sortThreadList,
  splitThreadList,
} from './thread-list'

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

describe('nextThreadAfterSend', () => {
  const row = (
    id: string,
    activityAtMs: number,
    urgency: 'attention' | 'quiet' = 'attention',
    workspace: object | null = {},
  ) => ({ id, activityAtMs, urgency, workspace })
  const items = [
    row('current', 1),
    row('old', 2),
    row('mid', 5),
    row('new', 8),
    row('scratch', 0, 'quiet', null),
    row('mine', 9, 'quiet'),
  ]
  const unread = new Set(['old', 'mid'])
  const isUnread = (item: { id: string }) => unread.has(item.id)

  it('goes to whatever has waited longest, never the thread you are on', () => {
    expect(nextThreadAfterSend(items, 'current', 'oldest', isUnread)?.id).toBe(
      'old',
    )
  })

  it('goes to the unread thread that moved last', () => {
    expect(
      nextThreadAfterSend(items, 'current', 'newestUnread', isUnread)?.id,
    ).toBe('mid')
  })

  it('stays put when nothing else needs you', () => {
    expect(
      nextThreadAfterSend(items, 'current', 'newestUnread', () => false),
    ).toBeNull()
    expect(
      nextThreadAfterSend([row('current', 1)], 'current', 'oldest', isUnread),
    ).toBeNull()
  })
})
