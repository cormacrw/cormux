import { commands } from '$lib/ipc'
import { showToast } from '$lib/feedback/show-toast'
import { leaveConversation } from './conversation-motion'

export async function startNewSession(threadId: string) {
  const restore = await leaveConversation()
  const result = await commands.newThreadSession(threadId)
  if (result.status === 'ok') return
  restore()
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
