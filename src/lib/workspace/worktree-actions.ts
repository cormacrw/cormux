import { commands } from '$lib/ipc'
import { toastCoreError } from '$lib/feedback/wire-feedback'
import { openWorkspaceThread } from '$lib/nav/open'

export async function openWorkspaceTerminal(workspaceId: string) {
  const result = await commands.openWorkspaceTerminal(workspaceId)
  if (result.status === 'error') toastCoreError(result.error)
}

/** Opens the worktree in the editor picked in Settings (Zed by default). */
export async function openWorkspaceEditor(workspaceId: string) {
  const result = await commands.openWorkspaceEditor(workspaceId)
  if (result.status === 'error') toastCoreError(result.error)
}

/** Starts a Reviewer on the workspace's own branch and opens its thread. */
export async function startBranchReview(workspaceId: string) {
  const result = await commands.startBranchReview(workspaceId)
  if (result.status === 'error') {
    toastCoreError(result.error)
    return
  }
  openWorkspaceThread(workspaceId, result.data)
}
