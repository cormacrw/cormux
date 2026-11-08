import type { ComposerAttachment } from '$lib/composer/attachments'
import { toastCoreError } from '$lib/feedback/wire-feedback'
import {
  threadTimeline,
  type ThreadActionError,
} from '$lib/thread/timeline-store.svelte'
import type { Conversation } from './conversations'

function toastFailure(failure: ThreadActionError | null): boolean {
  if (failure) toastCoreError(failure.error, failure.doing)
  return failure === null
}

/** Sends a message to the thread's agent; resolves false (after toasting) if it didn't send. */
export async function sendToThread(
  threadId: string,
  text: string,
  attachments: ComposerAttachment[] = [],
): Promise<boolean> {
  return toastFailure(await threadTimeline.send(threadId, text, attachments))
}

export async function stopThread(threadId: string) {
  toastFailure(await threadTimeline.stop(threadId))
}

/** Starts a fresh session in the thread, or goes back to an earlier conversation. */
export async function switchConversation(
  threadId: string,
  to: Conversation | 'new',
) {
  toastFailure(await threadTimeline.switchConversation(threadId, to))
}
