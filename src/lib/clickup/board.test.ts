import { describe, expect, it } from 'vitest'
import type {
  ClickupBoard,
  ClickupStatus,
  ClickupTask,
} from '$lib/ipc/bindings'
import {
  daysLeft,
  inProgressTasks,
  taskRef,
  parsePoints,
  sprintLanes,
  sprintRange,
  sprintTotals,
  withTaskStatus,
} from './board'

const status = (name: string, kind: string, order: number): ClickupStatus => ({
  name,
  color: null,
  kind,
  order,
})

const task = (
  id: string,
  statusName: string,
  points: number | null,
  assignee?: number,
): ClickupTask => ({
  id,
  customId: null,
  name: `Task ${id}`,
  status: statusName,
  statusColor: null,
  points,
  assignees: assignee
    ? [{ id: assignee, username: 'Sam', initials: 'S', color: null }]
    : [],
  priority: null,
  priorityColor: null,
  parent: null,
  url: '',
})

const statuses = [
  status('to do', 'open', 0),
  status('in progress', 'custom', 1),
  status('review', 'custom', 2),
  status('complete', 'closed', 3),
]

const board: ClickupBoard = {
  sprint: { id: 'l1', name: 'Sprint 12', startMs: null, dueMs: null },
  statuses,
  tasks: [
    task('a', 'to do', 3, 1),
    task('b', 'In Progress', null, 1),
    task('c', 'review', 2, 2),
    task('d', 'review', 5, 1),
  ],
  userId: 1,
}

describe('sprintLanes', () => {
  it('groups tasks by status, ignoring case', () => {
    const lanes = sprintLanes(statuses, board.tasks)
    expect(lanes.map((lane) => lane.tasks.map((t) => t.id))).toEqual([
      ['a'],
      ['b'],
      ['c', 'd'],
      [],
    ])
  })
})

describe('inProgressTasks', () => {
  it("keeps every task in a between status, the key owner's first", () => {
    expect(inProgressTasks(board).map((t) => t.id)).toEqual(['b', 'd', 'c'])
    expect(inProgressTasks(null)).toEqual([])
  })
})

describe('withTaskStatus', () => {
  it('moves one task without touching the rest', () => {
    const next = withTaskStatus(board, 'a', statuses[3]!)
    expect(next.tasks.find((t) => t.id === 'a')?.status).toBe('complete')
    expect(board.tasks[0]!.status).toBe('to do')
  })
})

describe('sprintTotals', () => {
  it('sums points and counts unpointed tasks', () => {
    expect(sprintTotals(board.tasks)).toEqual({ points: 10, unpointed: 1 })
  })
})

describe('parsePoints', () => {
  it('accepts zero and decimals, rejects junk', () => {
    expect(parsePoints('0')).toBe(0)
    expect(parsePoints(' 2.5 ')).toBe(2.5)
    expect(parsePoints('')).toBeNull()
    expect(parsePoints('-1')).toBeNull()
    expect(parsePoints('abc')).toBeNull()
  })
})

describe('sprint dates', () => {
  it('formats the range and counts days left', () => {
    const start = Date.UTC(2026, 8, 22, 12)
    const due = Date.UTC(2026, 9, 5, 12)
    expect(sprintRange(start, due)).toBe('Sep 22 – Oct 5')
    expect(sprintRange(null, due)).toBeNull()
    expect(daysLeft(due, due - 2.5 * 86_400_000)).toBe(3)
    expect(daysLeft(due, due + 1)).toBe(0)
  })
})

describe('taskRef', () => {
  it('falls back to the ClickUp ID without a custom one', () => {
    expect(taskRef({ id: '86abc', customId: 'ENG-1' })).toBe('ENG-1')
    expect(taskRef({ id: '86abc', customId: null })).toBe('86abc')
  })
})
