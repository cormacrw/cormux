import { describe, expect, it } from 'vitest'
import {
  commandMatchesQuery,
  filterCommands,
  groupFilteredCommands,
} from './filter'
import type { PaletteCommand } from './types'

const sample: PaletteCommand[] = [
  {
    id: 'a',
    group: 'Actions',
    label: 'Go to Homebase',
    run: () => {},
  },
  {
    id: 'b',
    group: 'Workspaces',
    label: 'Open Auth',
    meta: 'Idle',
    run: () => {},
  },
  {
    id: 'c',
    group: 'App',
    label: 'Run app in Auth',
    meta: 'pnpm dev',
    run: () => {},
  },
]

const home = sample[0]!
const idleWs = sample[1]!
const runApp = sample[2]!

describe('commandMatchesQuery', () => {
  it('matches label substring case-insensitively', () => {
    expect(commandMatchesQuery(home, 'home')).toBe(true)
    expect(commandMatchesQuery(home, 'HOMEBASE')).toBe(true)
  })

  it('matches meta substring', () => {
    expect(commandMatchesQuery(runApp, 'pnpm')).toBe(true)
    expect(commandMatchesQuery(idleWs, 'idle')).toBe(true)
  })

  it('returns all for empty query', () => {
    expect(commandMatchesQuery(home, '')).toBe(true)
    expect(commandMatchesQuery(home, '   ')).toBe(true)
  })
})

describe('filterCommands', () => {
  it('filters the list', () => {
    expect(filterCommands(sample, 'idle').map((c) => c.id)).toEqual(['b'])
  })
})

describe('groupFilteredCommands', () => {
  it('preserves spec group order and omits empty groups', () => {
    const grouped = groupFilteredCommands(sample)
    expect(grouped.map((g) => g.group)).toEqual([
      'Actions',
      'Workspaces',
      'App',
    ])
  })
})
