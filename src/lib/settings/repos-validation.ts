export function repoDomKey(id: string) {
  return id.replace(/[^\w-]/g, '-')
}

export function workspaceCountForRepo(
  repoId: string,
  workspaces: { repoId: string }[],
) {
  return workspaces.filter((row) => row.repoId === repoId).length
}
