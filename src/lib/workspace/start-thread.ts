import { toastCoreError } from '$lib/feedback/wire-feedback'
import { commands, fetchSnapshot } from '$lib/ipc'
import { draftBranchName, uniqueBranchName } from '$lib/new-workspace/draft'
import {
  hydrateFromSnapshot,
  repos,
  settings,
  workspaceRecords,
} from '$lib/state'

/**
 * Spawns a thread: a new worktree on a branch drafted from the prompt, with the prompt as its
 * first message. The core names it and sets it up in the background. Resolves the new
 * workspace's id, or null (after toasting) if it couldn't start.
 */
export async function startThread(input: {
  repoId: string
  /** Null branches off the repo's default branch. */
  base: string | null
  prompt: string
  /** Null leaves the engine's default. */
  model?: string | null
}): Promise<string | null> {
  const { repoId, prompt } = input
  const listed = await commands.listRepoBranches(repoId)
  const taken = [
    ...(listed.status === 'ok' ? listed.data.branches : []),
    ...workspaceRecords.records
      .filter((row) => row.repoId === repoId)
      .map((row) => row.branch),
  ]
  const result = await commands.createWorkspace({
    repoId,
    name: '',
    branch: uniqueBranchName(draftBranchName(prompt), taken),
    base:
      input.base?.trim() ||
      repos.getById(repoId)?.defaultBranch?.trim() ||
      'main',
    engine: settings.defaultEngine,
    goal: prompt,
    model: input.model ?? null,
  })
  if (result.status === 'error') {
    toastCoreError(result.error)
    return null
  }
  hydrateFromSnapshot(await fetchSnapshot())
  return result.data.workspaceId
}
