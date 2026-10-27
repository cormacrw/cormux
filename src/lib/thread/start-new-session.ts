import { commands } from '$lib/ipc'
import { showToast } from '$lib/feedback/show-toast'
import { appearance } from '$lib/state/appearance.svelte'
import { leaveConversation } from './conversation-motion'

export async function startNewSession(threadId: string) {
  // The wordmark starts rising back at once, not after the swap lands.
  appearance.glideWordmark()
  appearance.wordmarkAway = false
  const restore = await leaveConversation()
  const result = await commands.newThreadSession(threadId)
  if (result.status === 'ok') return
  restore()
  appearance.wordmarkAway = true
  showToast({
    tone: 'bad',
    parts: [
      {
        type: 'text',
        value:
          typeof result.error.message === 'string'
            ? result.error.message
            : 'Could not start a new session',
      },
    ],
  })
}
