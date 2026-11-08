import { describe, expect, it } from 'vitest'
import { resolveDefaultRepoId } from './settings-defaults'

describe('resolveDefaultRepoId', () => {
  it('resolves the default repo, falling back to the first', () => {
    const repos = [{ id: 'a' }, { id: 'b' }]
    expect(resolveDefaultRepoId(repos, 'b')).toBe('b')
    expect(resolveDefaultRepoId(repos, 'gone')).toBe('a')
    expect(resolveDefaultRepoId(repos, '')).toBe('a')
    expect(resolveDefaultRepoId([], 'b')).toBe('')
  })
})
