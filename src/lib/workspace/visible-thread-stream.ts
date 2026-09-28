import type { AgentChunk } from '$lib/ipc'
import { subscribeAgentChunks } from '$lib/ipc'

let listenGeneration = 0

/**
 * Subscribe to agent chunks only while `threadId` is the visible conversation.
 * Older subscriptions are ignored when the generation advances (COR-121).
 */
export function bindVisibleThreadStream(
  threadId: string,
  onChunk: (chunk: AgentChunk) => void,
): () => void {
  const generation = ++listenGeneration
  const stop = subscribeAgentChunks(threadId, (chunk) => {
    if (generation !== listenGeneration) return
    if (chunk.threadId !== threadId) return
    onChunk(chunk)
  })
  return () => {
    stop()
    if (generation === listenGeneration) listenGeneration += 1
  }
}
