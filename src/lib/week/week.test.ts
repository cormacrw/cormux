import { describe, expect, it } from 'vitest'
import type { ClickupBoard, ClickupTask, TodoRow } from '$lib/ipc/bindings'
import {
  addDays,
  dayLabel,
  isoDay,
  localDayOf,
  weekDays,
  weekRange,
  weekStart,
} from './dates'
import { byDay, loadingItems, plannedDone, weekItems } from './items'

describe('week dates', () => {
  it('starts weeks on Monday', () => {
    expect(weekStart('2026-10-23')).toBe('2026-10-19') // Friday
    expect(weekStart('2026-10-19')).toBe('2026-10-19') // Monday
    expect(weekStart('2026-10-25')).toBe('2026-10-19') // Sunday
    expect(weekDays('2026-10-19')).toEqual([
      '2026-10-19',
      '2026-10-20',
      '2026-10-21',
      '2026-10-22',
      '2026-10-23',
      '2026-10-24',
      '2026-10-25',
    ])
  })

  it('steps across months and years', () => {
    expect(addDays('2026-10-31', 1)).toBe('2026-11-01')
    expect(addDays('2027-01-01', -1)).toBe('2026-12-31')
    expect(weekStart('2027-01-01')).toBe('2026-12-28')
  })

  it('labels the range and each day', () => {
    expect(weekRange('2026-10-19')).toBe('Oct 19 – 25')
    expect(weekRange('2026-10-26')).toBe('Oct 26 – Nov 1')
    expect(dayLabel('2026-10-23')).toEqual({
      name: 'Fri',
      long: 'Friday',
      date: 'Oct 23',
    })
  })

  it('reads SQLite UTC stamps as a local day', () => {
    const stamp = '2026-10-23 23:30:00'
    expect(localDayOf(stamp)).toBe(
      isoDay(new Date(Date.UTC(2026, 9, 23, 23, 30))),
    )
    expect(localDayOf(null)).toBeNull()
    expect(localDayOf('nope')).toBeNull()
  })
})

const todo = (id: string, patch: Partial<TodoRow> = {}): TodoRow => ({
  id,
  number: 1,
  title: id,
  pinned: false,
  description: '',
  plannedOn: null,
  doneAt: null,
  ...patch,
})

const task = (id: string, status: string): ClickupTask => ({
  id,
  customId: `ENG-${id}`,
  name: id,
  status,
  statusColor: null,
  points: 3,
  assignees: [],
  priority: null,
  priorityColor: null,
  parent: null,
  url: '',
})

const board: ClickupBoard = {
  sprint: { id: 's', name: 'Sprint', startMs: null, dueMs: null },
  statuses: [
    { name: 'to do', color: null, kind: 'open', order: 0 },
    { name: 'complete', color: null, kind: 'closed', order: 1 },
  ],
  tasks: [task('t1', 'to do'), task('t2', 'complete'), task('t3', 'to do')],
  userId: 1,
}

const today = '2026-10-23'
const thisWeek = '2026-10-19'
const shown = weekDays(thisWeek)

describe('week items', () => {
  const items = weekItems(
    [
      todo('a'),
      todo('b', { plannedOn: '2026-10-20' }),
      todo('c', { doneAt: '2026-01-02 10:00:00' }),
      todo('d', { plannedOn: '2026-10-12' }),
      todo('e', { plannedOn: '2026-10-13', doneAt: '2026-10-13 10:00:00' }),
    ],
    board,
    [
      { taskId: 't3', plannedOn: '2026-10-23', doneAt: '2026-10-23 09:00:00' },
      { taskId: 't2', plannedOn: '2026-10-21', doneAt: null },
    ],
  )

  it('puts the newest TODOs first, then the sprint, with their plan', () => {
    expect(items.map((item) => item.key)).toEqual([
      'todo:e',
      'todo:d',
      'todo:c',
      'todo:b',
      'todo:a',
      'sprint:t1',
      'sprint:t2',
      'sprint:t3',
    ])
    const t2 = items.find((item) => item.id === 't2')!
    expect([t2.tag, t2.done, t2.doneInClickup]).toEqual(['ENG-t2', true, true])
    const t3 = items.find((item) => item.id === 't3')!
    expect([t3.plannedOn, t3.done]).toEqual(['2026-10-23', true])
  })

  it('loads open unplanned items and unfinished ones from past weeks', () => {
    const keys = (filter: 'all' | 'todo' | 'sprint') =>
      loadingItems(items, { today, thisWeek, shown, filter }).map(
        (item) => item.key,
      )
    // c was done long ago, e was done, t2 is finished in ClickUp.
    expect(keys('all')).toEqual(['todo:d', 'todo:a', 'sprint:t1'])
    expect(keys('todo')).toEqual(['todo:d', 'todo:a'])
    expect(keys('sprint')).toEqual(['sprint:t1'])
    // Looking at last week, d is on its day instead.
    expect(
      loadingItems(items, {
        today,
        thisWeek,
        shown: weekDays('2026-10-12'),
        filter: 'all',
      }).map((item) => item.key),
    ).toEqual(['todo:a', 'sprint:t1'])
  })

  it('keeps a task ticked off today in the loading area', () => {
    const ticked = weekItems(
      [todo('a', { doneAt: '2026-10-23 12:00:00' })],
      null,
      [],
    )
    ticked[0]!.doneOn = today
    expect(
      loadingItems(ticked, { today, thisWeek, shown, filter: 'all' }),
    ).toHaveLength(1)
  })

  it('groups by day and counts planned work done', () => {
    const lanes = byDay(items, shown)
    expect(lanes.get('2026-10-20')!.map((item) => item.key)).toEqual(['todo:b'])
    expect(lanes.get('2026-10-21')!.map((item) => item.key)).toEqual([
      'sprint:t2',
    ])
    expect(lanes.get('2026-10-24')).toEqual([])
    expect(plannedDone(items, shown)).toEqual({ done: 2, total: 3 })
  })
})
