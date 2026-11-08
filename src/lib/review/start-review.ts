import { openWorkspace } from '$lib/workspace/open'
import { toastCoreError } from '$lib/feedback/wire-feedback'
import { showToast } from '$lib/feedback/show-toast'
import { commands } from '$lib/ipc'
import type { PullRequest } from '$lib/state/prs.svelte'
import { workspaces } from '$lib/state'

/** Stays where you are; the core's toasts link to the workspace once it's up. */
async function startReviewWorkspace(repoId: string, prNumber: number) {
  const result = await commands.createReviewWorkspace({ repoId, prNumber })
  if (result.status === 'error') toastCoreError(result.error)
}

/** One review workspace per PR: reopens it rather than starting a duplicate. */
export function reviewPullRequest(pr: PullRequest) {
  const existing = workspaces.items.find(
    (row) =>
      row.kind === 'review' &&
      row.prNumber === pr.num &&
      (!pr.repoId || row.repoId === pr.repoId),
  )
  if (existing) openWorkspace(existing.id)
  else if (pr.repoId) void startReviewWorkspace(pr.repoId, pr.num)
  else
    showToast({
      tone: 'bad',
      parts: [
        {
          type: 'text',
          value: `#${pr.num} is in a repo Cormux doesn't have; add it in Settings to review it here.`,
        },
      ],
    })
}
