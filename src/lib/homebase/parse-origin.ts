/** Normalise a git remote URL to `owner/repo` for matching GitHub pull requests. */
export function parseOriginUrl(raw: string): string | null {
  const trimmed = raw.trim()
  if (!trimmed) return null

  if (trimmed.startsWith('git@github.com:')) {
    return normaliseSlug(
      stripGitSuffix(trimmed.slice('git@github.com:'.length)),
    )
  }
  if (trimmed.startsWith('ssh://git@github.com/')) {
    return normaliseSlug(
      stripGitSuffix(trimmed.slice('ssh://git@github.com/'.length)),
    )
  }

  const withoutScheme = trimmed.replace(/^https?:\/\//, '')
  const path = withoutScheme.replace(/^(www\.)?github\.com\//, '')
  return normaliseSlug(stripGitSuffix(path))
}

function stripGitSuffix(path: string): string {
  return path.replace(/\/$/, '').replace(/\.git$/, '')
}

function normaliseSlug(path: string): string | null {
  const parts = path.split('/').filter(Boolean)
  const owner = parts[0]
  const repo = parts[1]
  if (!owner || !repo) return null
  return `${owner.toLowerCase()}/${repo}`
}

export function matchRepoId(
  repoOrigins: Record<string, string>,
  prRepo: string,
): string | null {
  const needle = prRepo.toLowerCase()
  for (const [repoId, slug] of Object.entries(repoOrigins)) {
    if (slug.toLowerCase() === needle) return repoId
  }
  return null
}
