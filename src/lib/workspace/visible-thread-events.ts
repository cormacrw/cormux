import type { AgentEvent } from '$lib/ipc'
import { subscribeAgentEvents } from '$lib/ipc/channels'

let listenGeneration = 0

/**
 * Subscribe to full agent events while a thread tab is visible (COR-124).
 */
export function bindVisibleThreadEvents(
  threadId: string,
  onEvent: (event: AgentEvent) => void,
): () => void {
  const generation = ++listenGeneration
  const stop = subscribeAgentEvents(threadId, (event) => {
    if (generation !== listenGeneration) return
    onEvent(event)
  })
  return () => {
    stop()
    if (generation === listenGeneration) listenGeneration += 1
  }
}
