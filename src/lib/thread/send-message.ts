import { commands } from '$lib/ipc'
import {
  toPromptAttachment,
  type ComposerAttachment,
} from '$lib/composer/attachments'
import { showToast } from '$lib/feedback/show-toast'
import { threadTimeline } from '$lib/state/thread-timeline.svelte'

import { firstPromptTransition } from './conversation-motion'

/**
 * Post a user message to a thread's agent. The core shows it in the thread and moves the thread's
 * status; resolves false (after toasting) if it didn't send.
 */
export async function sendThreadMessage(
  threadId: string,
  text: string,
  attachments: ComposerAttachment[] = [],
): Promise<boolean> {
  firstPromptTransition(() => threadTimeline.userMessageShown(threadId))
  let message: string | null = null
  try {
    const result = await commands.sendThreadPrompt(
      threadId,
      text,
      attachments.map(toPromptAttachment),
    )
    if (result.status === 'error') {
      message =
        typeof result.error.message === 'string'
          ? result.error.message
          : 'Could not reach the agent'
    }
  } catch (error) {
    message =
      error instanceof Error ? error.message : 'Could not reach the agent'
  }
  if (message === null) return true
  showToast({ tone: 'bad', parts: [{ type: 'text', value: message }] })
  return false
}
