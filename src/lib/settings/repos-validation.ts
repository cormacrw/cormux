export type AddRepoValidationError =
  | 'empty'
  | 'invalid-path'
  | 'duplicate-path'
  | 'duplicate-name'

export function validateAddRepoPath(
  path: string,
  existing: { id: string; path: string }[],
): AddRepoValidationError | null {
  const trimmed = path.trim()
  if (!trimmed) return 'empty'
  const normalized = trimmed.replace(/\/+$/, '')
  const segment = normalized.split('/').filter(Boolean).pop() ?? ''
  if (!/^(~|\/)/.test(normalized) || !segment || segment === '~') {
    return 'invalid-path'
  }
  if (existing.some((repo) => repo.path === normalized)) {
    return 'duplicate-path'
  }
  const id = segment
  if (existing.some((repo) => repo.id === id)) {
    return 'duplicate-name'
  }
  return null
}

export function addRepoErrorMessage(
  code: AddRepoValidationError,
  name?: string,
): string {
  switch (code) {
    case 'empty':
      return 'Enter the path to a folder'
    case 'invalid-path':
      return 'Use a full path, like ~/code/my-project'
    case 'duplicate-path':
      return 'That folder is already added'
    case 'duplicate-name':
      return `A repo named ${name ?? 'that name'} is already added`
  }
}

export function repoDomKey(id: string) {
  return id.replace(/[^\w-]/g, '-')
}

export function workspaceCountForRepo(
  repoId: string,
  workspaces: { repoId: string }[],
) {
  return workspaces.filter((row) => row.repoId === repoId).length
}

export function removeRepoBlockReason(
  repo: { id: string; name: string },
  repoCount: number,
  workspaceCount: number,
): string | null {
  if (workspaceCount > 0) {
    const tear = workspaceCount === 1 ? 'it' : 'them'
    return `${repo.name} has ${workspaceCount} workspace${workspaceCount === 1 ? '' : 's'}. Tear ${tear} down first.`
  }
  if (repoCount <= 1) {
    return 'Harness needs at least one repo'
  }
  return null
}
