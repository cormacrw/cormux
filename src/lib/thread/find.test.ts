import { describe, expect, it } from 'vitest'
import {
  findCounterText,
  findMatches,
  rowKey,
  rowSearchText,
  stepMatch,
} from './find'
import type { TimelineRow } from './timeline-types'

const rows: TimelineRow[] = [
  {
    kind: 'speaker',
    id: 'speaker-1',
    speaker: 'agent',
    role: 'Lead',
    engine: 'Claude Code',
    seq: 1,
    atMs: 0,
  },
  {
    kind: 'item',
    item: {
      kind: 'user',
      id: 'u1',
      text: 'Fix the Login bug',
      seq: 2,
      atMs: 0,
    },
  },
  {
    kind: 'item',
    item: {
      kind: 'toolRun',
      id: 'run-1',
      seq: 3,
      steps: [
        {
          kind: 'tool',
          id: 't1',
          icon: 'terminal',
          title: 'Ran',
          detail: 'pnpm test login',
          rawDetail: 'pnpm test login --run',
          seq: 3,
          atMs: 0,
        },
        {
          kind: 'edit',
          id: 'e1',
          path: 'src/login.ts',
          verb: 'Edited',
          seq: 4,
          atMs: 0,
        },
      ],
    },
  },
  {
    kind: 'item',
    item: {
      kind: 'thought',
      id: 'a1',
      text: 'The login form never awaits the request.',
      seq: 5,
      atMs: 0,
      role: 'agent',
    },
  },
]

describe('findMatches', () => {
  it('finds every occurrence, case-insensitively, including inside tool steps', () => {
    expect(findMatches(rows, '  LOGIN ')).toEqual([
      { row: 1, occurrence: 0 },
      { row: 2, occurrence: 0 },
      { row: 2, occurrence: 1 },
      { row: 2, occurrence: 2 },
      { row: 3, occurrence: 0 },
    ])
    expect(findMatches(rows, 'nothing here')).toEqual([])
    expect(findMatches(rows, '   ')).toEqual([])
  })

  it('skips speaker rows, which only label a turn', () => {
    expect(rowSearchText(rows[0]!)).toBe('')
    expect(findMatches(rows, 'Lead')).toEqual([])
  })

  it('keys rows the way the timeline renders them', () => {
    expect(rows.map(rowKey)).toEqual(['speaker-1', 'u1', 'run-1', 'a1'])
  })
})

describe('stepping and the counter', () => {
  it('wraps around both ends', () => {
    expect(stepMatch(4, 5, 1)).toBe(0)
    expect(stepMatch(0, 5, -1)).toBe(4)
    expect(stepMatch(0, 0, 1)).toBe(0)
  })

  it('counts from one', () => {
    expect(findCounterText(2, 12)).toBe('3 of 12')
    expect(findCounterText(0, 0)).toBe('No matches')
  })
})
