import { openWorkspace } from '$lib/workspace/open'
import { toastCoreError } from '$lib/feedback/wire-feedback'
import { commands } from '$lib/ipc'
import { refreshSnapshot } from '$lib/core/sync'
import type { PullRequest } from '$lib/state/prs.svelte'
import { workspaces } from '$lib/state'

async function startReviewWorkspace(pr: PullRequest): Promise<boolean> {
  const result = await commands.createReviewWorkspace({
    repoId: pr.repoId ?? 'my-app',
    prNumber: pr.num,
    title: pr.title,
    head: pr.head,
    base: pr.base,
    author: pr.author,
    authorIsYou: pr.author === 'you',
    filesChanged: pr.files,
    prHtmlUrl: pr.htmlUrl || null,
  })

  if (result.status === 'error') {
    toastCoreError(result.error)
    return false
  }

  // Stay where you are; the toasts link to the workspace once it's up.
  await refreshSnapshot()
  return true
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
  else void startReviewWorkspace(pr)
}
