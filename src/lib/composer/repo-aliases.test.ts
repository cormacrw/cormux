import { describe, expect, it } from 'vitest'
import {
  expandRepoAliases,
  filterRepos,
  insertRepoAlias,
  repoAlias,
  repoToken,
} from './repo-aliases'

const repos = [
  { name: 'cormux', path: '/Users/me/dev/cormux' },
  { name: 'web', path: '/Users/me/dev/web' },
  { name: 'web admin', path: '/Users/me/dev/web-admin' },
]

describe('repo aliases', () => {
  it('names a repo by its name as one word', () => {
    expect(repoAlias({ name: 'web admin' })).toBe('web-admin')
    expect(repoAlias({ name: 'cormux' })).toBe('cormux')
  })

  it('finds the @ word the caret is in, but not emails or packages', () => {
    expect(repoToken('look at @cor', 12)).toEqual({
      start: 8,
      end: 12,
      query: 'cor',
    })
    expect(repoToken('@', 1)).toEqual({ start: 0, end: 1, query: '' })
    expect(repoToken('mail me@example.com', 19)).toBeNull()
    expect(repoToken('install @tauri-apps/api', 23)).toBeNull()
    expect(repoToken('no at sign', 10)).toBeNull()
  })

  it('inserts the alias with a space after it', () => {
    expect(
      insertRepoAlias(
        'compare @co with this',
        { start: 8, end: 11, query: 'co' },
        'cormux',
      ),
    ).toEqual({ text: 'compare @cormux with this', caret: 16 })
    expect(
      insertRepoAlias('@w', { start: 0, end: 2, query: 'w' }, 'web'),
    ).toEqual({
      text: '@web ',
      caret: 5,
    })
  })

  it('lists aliases starting with the query first', () => {
    expect(filterRepos(repos, 'web').map((repo) => repo.name)).toEqual([
      'web',
      'web admin',
    ])
    expect(filterRepos(repos, 'dev/cor').map((repo) => repo.name)).toEqual([
      'cormux',
    ])
    expect(filterRepos(repos, '')).toHaveLength(3)
  })

  it('expands known aliases to their folders and leaves the rest', () => {
    expect(
      expandRepoAliases(
        'Port the toast from @cormux into @web-admin (and @Web). Mail me@web.com, see @unknown and @tauri-apps/api.',
        repos,
      ),
    ).toBe(
      'Port the toast from /Users/me/dev/cormux into /Users/me/dev/web-admin (and /Users/me/dev/web). Mail me@web.com, see @unknown and @tauri-apps/api.',
    )
    expect(expandRepoAliases('@web.ts stays', repos)).toBe('@web.ts stays')
    expect(expandRepoAliases('@cormux', [])).toBe('@cormux')
  })
})
