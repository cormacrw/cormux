import { describe, expect, it } from 'vitest'
import { expandRepoAliases, filterRepos, repoAlias } from './repo-aliases'

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
