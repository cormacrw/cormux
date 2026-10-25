import { commands } from '$lib/ipc'
import {
  shownMessage,
  toPromptAttachment,
  type ComposerAttachment,
} from '$lib/composer/attachments'
import { showToast } from '$lib/feedback/show-toast'
import { threads } from '$lib/state/threads.svelte'
import { threadTimeline } from '$lib/state/thread-timeline.svelte'

import { firstPromptTransition } from './conversation-motion'

/** Post a user message to a thread's agent. Resolves false (after toasting) if it didn't send. */
export async function sendThreadMessage(
  threadId: string,
  text: string,
  attachments: ComposerAttachment[] = [],
): Promise<boolean> {
  firstPromptTransition(() => {
    threadTimeline.appendStreamChunk(
      threadId,
      shownMessage(
        text,
        attachments.map((attachment) => attachment.name),
      ),
      'user',
    )
    threads.setStatus(threadId, 'running')
  })
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
  threads.setStatus(threadId, 'idle')
  showToast({ tone: 'bad', parts: [{ type: 'text', value: message }] })
  return false
}
