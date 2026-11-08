import { describe, expect, it } from 'vitest'
import { filterCommands, groupFilteredCommands } from './filter'
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

describe('filterCommands', () => {
  const ids = (query: string) => filterCommands(sample, query).map((c) => c.id)

  it('matches label and meta substrings, ignoring case', () => {
    expect(ids('HOMEBASE')).toEqual(['a'])
    expect(ids('idle')).toEqual(['b'])
    expect(ids('pnpm')).toEqual(['c'])
  })

  it('keeps everything for a blank query', () => {
    expect(ids('   ')).toEqual(['a', 'b', 'c'])
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
