import type { RepoRecord } from '$lib/ipc/bindings'
import { escapeRegExp, prefixFirst } from './match'

type AliasRepo = Pick<RepoRecord, 'name' | 'path'>

/** What `@` names a repo by: its name, with spaces as dashes so it stays one word. */
export function repoAlias(repo: Pick<RepoRecord, 'name'>): string {
  return repo.name.trim().replace(/\s+/g, '-')
}

export function filterRepos<T extends AliasRepo>(
  repos: T[],
  query: string,
): T[] {
  return prefixFirst(
    repos,
    query,
    (repo, needle) => repoAlias(repo).toLowerCase().startsWith(needle),
    (repo, needle) =>
      repo.name.toLowerCase().includes(needle) ||
      repo.path.toLowerCase().includes(needle),
  )
}

/**
 * The message with each `@alias` of a configured repo replaced by the repo's folder, so the
 * agent can work with it. Anything else after an `@` (emails, packages, unknown names) stays.
 */
export function expandRepoAliases(text: string, repos: AliasRepo[]): string {
  const byAlias = new Map(
    repos.map((repo) => [repoAlias(repo).toLowerCase(), repo.path]),
  )
  if (!byAlias.size) return text
  const aliases = [...byAlias.keys()].sort((a, b) => b.length - a.length)
  const pattern = new RegExp(
    `(^|[\\s(])@(${aliases.map(escapeRegExp).join('|')})(?![\\w-]|\\.\\w)`,
    'gi',
  )
  return text.replace(
    pattern,
    (_, before: string, alias: string) =>
      `${before}${byAlias.get(alias.toLowerCase())}`,
  )
}
