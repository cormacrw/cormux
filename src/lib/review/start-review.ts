import { commands } from '$lib/ipc'
import { fetchSnapshot } from '$lib/ipc'
import type { PullRequest } from '$lib/state/prs.svelte'
import { app, homebaseUi, hydrateFromSnapshot } from '$lib/state'

export async function startReviewWorkspace(pr: PullRequest): Promise<boolean> {
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
    return false
  }

  homebaseUi.resetFilter()
  hydrateFromSnapshot(await fetchSnapshot())
  app.openWorkspace(result.data.workspaceId)
  return true
}
