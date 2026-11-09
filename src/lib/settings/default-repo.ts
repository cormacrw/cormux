/** The saved default repo if it still exists, else the first repo. */
export function resolveDefaultRepoId(
  repos: { id: string }[],
  preferred: string,
): string {
  if (preferred && repos.some((repo) => repo.id === preferred)) return preferred
  return repos[0]?.id ?? ''
}
