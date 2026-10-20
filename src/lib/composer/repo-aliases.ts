import type { RepoRecord } from '$lib/ipc/bindings'

const MAX_SHOWN = 8

/** An `@alias` word in the draft: where it starts and ends, and what's typed before the caret. */
export type RepoToken = { start: number; end: number; query: string }

type AliasRepo = Pick<RepoRecord, 'name' | 'path'>

/** What `@` names a repo by: its name, with spaces as dashes so it stays one word. */
export function repoAlias(repo: Pick<RepoRecord, 'name'>): string {
  return repo.name.trim().replace(/\s+/g, '-')
}

/**
 * The `@` word the caret is in, when it starts the draft or follows whitespace, or null.
 * An email address (`me@example.com`) or a scoped package (`@tauri-apps/api`) doesn't count.
 */
export function repoToken(draft: string, caret: number): RepoToken | null {
  const match = /(^|\s)@([\w.-]*)$/.exec(draft.slice(0, caret))
  if (!match) return null
  const rest = /^\S*/.exec(draft.slice(caret))?.[0] ?? ''
  if (!/^[\w.-]*$/.test(rest)) return null
  return {
    start: match.index + (match[1]?.length ?? 0),
    end: caret + rest.length,
    query: match[2] ?? '',
  }
}

/** The draft with `token` replaced by `@alias `, and where the caret goes after it. */
export function insertRepoAlias(
  draft: string,
  token: RepoToken,
  alias: string,
): { text: string; caret: number } {
  const rest = draft.slice(token.end)
  const inserted = `@${alias}${/^\s/.test(rest) ? '' : ' '}`
  return {
    text: draft.slice(0, token.start) + inserted + rest,
    caret: token.start + alias.length + 2,
  }
}

/** Aliases starting with the query first, then names or paths containing it. */
export function filterRepos<T extends AliasRepo>(
  repos: T[],
  query: string,
): T[] {
  const needle = query.toLowerCase()
  const starts = repos.filter((repo) =>
    repoAlias(repo).toLowerCase().startsWith(needle),
  )
  const contains = repos.filter(
    (repo) =>
      !starts.includes(repo) &&
      (repo.name.toLowerCase().includes(needle) ||
        repo.path.toLowerCase().includes(needle)),
  )
  return [...starts, ...contains].slice(0, MAX_SHOWN)
}

function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
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
  // Longest first, so `@web-admin` isn't read as `@web` followed by `-admin`.
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
