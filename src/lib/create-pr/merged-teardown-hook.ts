import type { PullRequest } from '$lib/state/prs.svelte'
import type { Workspace } from '$lib/state/workspaces.svelte'
import { CREATE_PR_GAPS } from './gaps'

/**
 * COR-175 / COR-26: when Settings enables teardown after merge and a workspace PR
 * disappears from the open list, teardown should run. Not wired yet — tracked in gaps.
 */
export function workspacesAwaitingMergeTeardown(
  workspaces: Workspace[],
  openPrs: PullRequest[],
): string[] {
  void CREATE_PR_GAPS
  const openNums = new Set(openPrs.map((row) => row.num))
  return workspaces
    .filter((row) => row.prNumber != null && !openNums.has(row.prNumber))
    .map((row) => row.id)
}
