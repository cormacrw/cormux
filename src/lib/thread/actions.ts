// What the user does to a thread, with the motion around it: the store only talks to the core.
import type { ComposerAttachment } from '$lib/composer/attachments'
import { toastCoreError } from '$lib/feedback/wire-feedback'
import { wordmark } from '$lib/background/wordmark.svelte'
import {
  threadTimeline,
  type ThreadActionError,
} from '$lib/thread/timeline-store.svelte'
import type { ConversationSummary } from '$lib/ipc/bindings'
import { firstPromptTransition, leaveConversation } from './conversation-motion'

function toastFailure(failure: ThreadActionError | null): boolean {
  if (failure) toastCoreError(failure.error, failure.doing)
  return failure === null
}

/**
 * Sends a message to the thread's agent; resolves false (after toasting) if it didn't send.
 * A first message takes the greeting away as it lands.
 */
export async function sendToThread(
  threadId: string,
  text: string,
  attachments: ComposerAttachment[] = [],
): Promise<boolean> {
  firstPromptTransition(
    () => threadTimeline.userMessageShown(threadId),
    () => wordmark.glide(),
  )
  return toastFailure(await threadTimeline.send(threadId, text, attachments))
}

export async function stopThread(threadId: string) {
  toastFailure(await threadTimeline.stop(threadId))
}

/**
 * Starts a fresh session in the thread, or goes back to an earlier conversation. The one on
 * screen fades out first, and comes back if the swap fails.
 */
export async function switchConversation(
  threadId: string,
  to: ConversationSummary | 'new',
) {
  // For a new session the wordmark starts rising back at once, not after the swap lands.
  const stayDown = to === 'new' ? wordmark.riseEarly() : undefined
  const restore = await leaveConversation()
  const failure = await threadTimeline.switchConversation(threadId, to)
  if (failure) {
    restore()
    stayDown?.()
  }
  toastFailure(failure)
}
