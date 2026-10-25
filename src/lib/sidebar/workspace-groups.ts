/** A repo's workspaces in the sidebar, in its own section. */
export type WorkspaceGroup<W> = {
  repoId: string
  /** The repo's name, or null for workspaces whose repo isn't known. */
  repoName: string | null
  workspaces: W[]
}

/**
 * Workspaces grouped by repo, repos in their own order (unknown last), each
 * group keeping the workspaces' order. Repos without workspaces are left out.
 */
export function groupWorkspacesByRepo<W extends { id: string }>(
  workspaces: W[],
  repoIdFor: (workspaceId: string) => string | undefined,
  repos: { id: string; name: string }[],
): WorkspaceGroup<W>[] {
  const byRepo = new Map<string, W[]>()
  for (const workspace of workspaces) {
    const repoId = repoIdFor(workspace.id) ?? ''
    byRepo.set(repoId, [...(byRepo.get(repoId) ?? []), workspace])
  }
  const groups: WorkspaceGroup<W>[] = []
  for (const repo of repos) {
    const list = byRepo.get(repo.id)
    if (!list) continue
    groups.push({ repoId: repo.id, repoName: repo.name, workspaces: list })
    byRepo.delete(repo.id)
  }
  const rest = [...byRepo.values()].flat()
  if (rest.length) groups.push({ repoId: '', repoName: null, workspaces: rest })
  return groups
}
