import { commands } from '$lib/ipc'
import { showToast } from '$lib/feedback/show-toast'
import { threads } from '$lib/state/threads.svelte'
import { threadTimeline } from '$lib/state/thread-timeline.svelte'

/** Post a user message to a thread's agent. Resolves false (after toasting) if it didn't send. */
export async function sendThreadMessage(threadId: string, text: string): Promise<boolean> {
  threadTimeline.appendStreamChunk(threadId, text, 'user')
  threads.setStatus(threadId, 'running')
  let message: string | null = null
  try {
    const result = await commands.sendThreadPrompt(threadId, text)
    if (result.status === 'error') {
      message =
        typeof result.error.message === 'string'
          ? result.error.message
          : 'Could not reach the agent'
    }
  } catch (error) {
    message = error instanceof Error ? error.message : 'Could not reach the agent'
  }
  if (message === null) return true
  threads.setStatus(threadId, 'idle')
  showToast({ tone: 'bad', parts: [{ type: 'text', value: message }] })
  return false
}
