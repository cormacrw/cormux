import { commands } from '$lib/ipc'
import { showToast } from '$lib/feedback/show-toast'
import type { Conversation } from './conversations'
import { leaveConversation } from './conversation-motion'

/** Switches the thread back to an earlier conversation; the agent resumes its session. */
export async function resumeConversation(
  threadId: string,
  conversation: Conversation,
) {
  const restore = await leaveConversation()
  const result = await commands.resumeThreadSession({
    threadId,
    conversationId: conversation.id,
    sessionId: conversation.sessionId,
  })
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
            : 'Could not resume that conversation',
      },
    ],
  })
}
