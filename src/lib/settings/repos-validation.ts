import { plural, pluralWord } from '$lib/format/plural'

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
    return `${repo.name} has ${plural(workspaceCount, 'workspace')}. Tear ${pluralWord(workspaceCount, 'it', 'them')} down first.`
  }
  if (repoCount <= 1) {
    return 'Harness needs at least one repo'
  }
  return null
}
