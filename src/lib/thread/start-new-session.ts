import { commands } from '$lib/ipc'
import { showToast } from '$lib/feedback/show-toast'

export async function startNewSession(threadId: string) {
  const result = await commands.newThreadSession(threadId)
  if (result.status === 'ok') return
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
