import { prs } from '$lib/state'
import type { Workspace } from '$lib/state/workspaces.svelte'
import { stacks } from './stacks.svelte'
import { branchPullRequest, type BranchPr } from './branch-pr'

/** The PR for one of a workspace's branches (its checked-out one by default); every view asks here. */
export function workspacePr(
  workspace: Workspace,
  branch: string = workspace.branch,
): BranchPr | null {
  const opened =
    branch === workspace.branch && workspace.prNumber != null
      ? { number: workspace.prNumber, url: workspace.prHtmlUrl }
      : null
  return branchPullRequest({
    items: prs.items,
    repoId: workspace.repoId,
    branch,
    opened,
    stackPr: stacks.branchFor(workspace.id, branch)?.pr,
  })
}

/** A PR comes from a workspace's branch once; review workspaces submit a review instead. */
export function canCreatePr(workspace: Workspace): boolean {
  return workspace.kind !== 'review' && workspacePr(workspace) == null
}
