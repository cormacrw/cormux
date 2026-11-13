// What the user does to a thread, with the motion around it: the store only talks to the core.
import type { ComposerAttachment } from '$lib/composer/attachments'
import { toastFailure } from '$lib/feedback/action-failure'
import { wordmark } from '$lib/background/wordmark.svelte'
import { threadTimeline } from '$lib/thread/timeline-store.svelte'
import { snoozes } from '$lib/sidebar/snooze-store.svelte'
import { threads } from '$lib/state/threads.svelte'
import type { ConversationSummary } from '$lib/ipc/bindings'
import { firstPromptTransition, leaveConversation } from './conversation-motion'

/**
 * Sends a message to the thread's agent; resolves false (after toasting) if it didn't send.
 * A first message takes the greeting away as it lands. Writing to a snoozed thread means you're
 * on it again, so it comes back to the list.
 */
export async function sendToThread(
  threadId: string,
  text: string,
  attachments: ComposerAttachment[] = [],
): Promise<boolean> {
  const sending = threadTimeline.send(threadId, text, attachments)
  const shown = threadTimeline.userMessageShown(threadId, sending)
  firstPromptTransition(
    () => shown,
    () => wordmark.glide(),
  )
  const sent = toastFailure(await sending)
  if (sent) wakeThread(threadId)
  return sent
}

/** Ends the snooze on the thread's row in the sidebar, which is its workspace's. */
export function wakeThread(threadId: string) {
  const workspaceId = threads.getById(threadId)?.workspaceId
  if (workspaceId) snoozes.unsnooze(workspaceId)
}

export async function stopThread(threadId: string) {
  toastFailure(await threadTimeline.stop(threadId))
}

/** Pauses the thread's agent, or lets a paused one carry on. */
export async function setThreadPaused(threadId: string, paused: boolean) {
  toastFailure(await threadTimeline.setPaused(threadId, paused))
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
