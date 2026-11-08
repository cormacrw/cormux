import type { WorkspaceKind } from '$lib/ipc/bindings'

/** Review workspaces show it once the review is in; others once a branch review found something. */
export function showFindingsTab(input: {
  workspaceKind: WorkspaceKind
  reviewReady: boolean
  hasFindings: boolean
}): boolean {
  if (input.workspaceKind === 'review') return input.reviewReady
  return input.hasFindings
}
