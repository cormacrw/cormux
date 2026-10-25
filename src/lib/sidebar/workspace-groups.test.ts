import { describe, expect, it } from 'vitest'
import { groupWorkspacesByRepo } from './workspace-groups'

describe('groupWorkspacesByRepo', () => {
  it('groups by repo in repo order, keeping workspace order, unknown last', () => {
    const repoOf: Record<string, string> = {
      a: 'r2',
      b: 'r1',
      c: 'r2',
      d: 'gone',
    }
    const groups = groupWorkspacesByRepo(
      [{ id: 'a' }, { id: 'b' }, { id: 'c' }, { id: 'd' }, { id: 'e' }],
      (id) => repoOf[id],
      [
        { id: 'r1', name: 'api' },
        { id: 'r2', name: 'web' },
        { id: 'r3', name: 'empty' },
      ],
    )
    expect(
      groups.map((group) => [
        group.repoName,
        group.workspaces.map((w) => w.id),
      ]),
    ).toEqual([
      ['api', ['b']],
      ['web', ['a', 'c']],
      [null, ['d', 'e']],
    ])
  })
})
