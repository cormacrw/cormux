import { commands } from '$lib/ipc'
import { changesReview, type FileReviewState } from './review-store.svelte'

export async function applyFileReview(
  workspaceId: string,
  path: string,
  decision: 'approve' | 'reject' | 'undo',
) {
  if (decision === 'undo') {
    changesReview.setReview(workspaceId, path, 'pending')
    return
  }
  const result = await commands.reviewWorktreeFile(workspaceId, path, decision)
  if (result.status === 'error') {
    throw new Error(JSON.stringify(result.error))
  }
  const next: FileReviewState = decision === 'approve' ? 'approved' : 'rejected'
  if (decision === 'reject') {
    changesReview.clearPath(workspaceId, path)
  } else {
    changesReview.setReview(workspaceId, path, next)
  }
}
