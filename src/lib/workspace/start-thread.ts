import {
  toPromptAttachment,
  type ComposerAttachment,
} from '$lib/composer/attachments'
import { toastCoreError } from '$lib/feedback/wire-feedback'
import { commands } from '$lib/ipc'
import { refreshSnapshot } from '$lib/core/sync'

/**
 * Spawns a thread: a new worktree off the repo's default branch, with the prompt as its first
 * message. If the prompt asks for other work to start from, the agent brings it in. The core
 * names the workspace and its branch and sets it up in the background. Resolves the new
 * workspace's id, or null (after toasting) if it couldn't start.
 */
export async function startThread(input: {
  repoId: string
  prompt: string
  /** Null leaves the engine's default. */
  model?: string | null
  /** Null leaves the engine's default. */
  effort?: string | null
  /** Sent with the prompt as the first message's images and files. */
  attachments?: ComposerAttachment[]
}): Promise<string | null> {
  const { repoId, prompt } = input
  const result = await commands.createWorkspace({
    repoId,
    name: '',
    branch: '',
    base: null,
    engine: null,
    goal: prompt,
    model: input.model ?? null,
    effort: input.effort ?? null,
    attachments: (input.attachments ?? []).map(toPromptAttachment),
  })
  if (result.status === 'error') {
    toastCoreError(result.error)
    return null
  }
  await refreshSnapshot()
  return result.data.workspaceId
}
