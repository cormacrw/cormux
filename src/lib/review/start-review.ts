import { openWorkspace } from '$lib/nav/open'
import { toastCoreError } from '$lib/feedback/wire-feedback'
import { showToast } from '$lib/feedback/show-toast'
import { commands } from '$lib/ipc'
import type { PullRequest } from '$lib/state/prs.svelte'

/**
 * Reviews the PR in its own workspace, which the core finds or starts (one per PR). An existing
 * one opens; a new one stays in the background, and the core's toasts link to it once it's up.
 */
export async function reviewPullRequest(pr: PullRequest) {
  if (!pr.repoId) {
    showToast({
      tone: 'bad',
      parts: [
        {
          type: 'text',
          value: `#${pr.num} is in a repo Cormux doesn't have; add it in Settings to review it here.`,
        },
      ],
    })
    return
  }
  const result = await commands.createReviewWorkspace({
    repoId: pr.repoId,
    prNumber: pr.num,
  })
  if (result.status === 'error') toastCoreError(result.error)
  else if (!result.data.created) openWorkspace(result.data.workspaceId)
}
