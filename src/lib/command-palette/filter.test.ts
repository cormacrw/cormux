import { describe, expect, it } from 'vitest'
import {
  filterCommands,
  groupFilteredCommands,
  matchedReference,
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

describe('filterCommands', () => {
  const ids = (query: string) => filterCommands(sample, query).map((c) => c.id)

  it('matches label and meta substrings, ignoring case', () => {
    expect(ids('HOMEBASE')).toEqual(['a'])
    expect(ids('idle')).toEqual(['b'])
    expect(ids('pnpm')).toEqual(['c'])
  })

  it('finds a PR by its number, with or without the #', () => {
    const pr: PaletteCommand[] = [
      ...sample,
      {
        id: 'd',
        group: 'Workspaces',
        label: 'Open Billing',
        meta: 'PR #482 · Idle',
        run: () => {},
      },
    ]
    for (const query of ['482', '#482', 'pr 482', 'PR #482']) {
      expect(filterCommands(pr, query).map((c) => c.id)).toEqual(['d'])
    }
  })

  it('finds a thread by a reference’s name or URL, and says which one matched', () => {
    const thread: PaletteCommand = {
      id: 'e',
      group: 'Workspaces',
      label: 'Open Billing',
      meta: 'Idle',
      references: [
        { name: 'Billing spec', url: 'https://docs.google.com/d/1' },
        {
          name: 'CU-86b2 Refund flow',
          url: 'https://app.clickup.com/t/86b2abc',
        },
      ],
      run: () => {},
    }
    const commands = [...sample, thread]
    for (const query of ['cu-86b2', '86b2abc', 'refund']) {
      expect(filterCommands(commands, query).map((c) => c.id)).toEqual(['e'])
      expect(matchedReference(thread, query)).toBe('CU-86b2 Refund flow')
    }
    // The label matched on its own, so no reference is named.
    expect(matchedReference(thread, 'billing')).toBeUndefined()
    expect(matchedReference(thread, '')).toBeUndefined()
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
