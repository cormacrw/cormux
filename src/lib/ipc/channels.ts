import { Channel } from '@tauri-apps/api/core'
import {
  commands,
  type AgentChunk,
  type AgentEvent,
  type DiffUpdate,
  type Error as CoreError,
  type PtyChunk,
  type Result,
} from './bindings'

/**
 * Subscribe helpers for high-volume ordered streams. The core owns the bytes;
 * the webview only listens while a surface is visible. Each returns a stop
 * function that ends the core task behind the channel.
 */
function subscribe<T>(
  start: (channel: Channel<T>) => Promise<Result<number, CoreError>>,
  onMessage: (message: T) => void,
): () => void {
  const channel = new Channel<T>()
  let stopped = false
  channel.onmessage = (message) => {
    if (!stopped) onMessage(message)
  }
  const id = start(channel).then(
    (result) => (result.status === 'ok' ? result.data : null),
    () => null,
  )
  return () => {
    if (stopped) return
    stopped = true
    void id.then((value) => {
      if (value !== null) void commands.unsubscribe(value)
    })
  }
}

export function subscribeAgentChunks(
  threadId: string,
  onChunk: (chunk: AgentChunk) => void,
) {
  return subscribe((channel) => commands.subscribeAgentChunks(threadId, channel), onChunk)
}

export function subscribeAgentEvents(
  threadId: string,
  onEvent: (event: AgentEvent) => void,
) {
  return subscribe((channel) => commands.subscribeAgentEvents(threadId, channel), onEvent)
}

export function subscribePty(
  workspaceId: string,
  onChunk: (chunk: PtyChunk) => void,
) {
  return subscribe((channel) => commands.subscribePty(workspaceId, channel), onChunk)
}

export function subscribeDiffs(
  workspaceId: string,
  onUpdate: (update: DiffUpdate) => void,
) {
  return subscribe((channel) => commands.subscribeDiffs(workspaceId, channel), onUpdate)
}

export async function startStreamingSpike(
  onAgent: (chunk: AgentChunk) => void,
  onPty: (chunk: PtyChunk) => void,
) {
  const agent = new Channel<AgentChunk>()
  const pty = new Channel<PtyChunk>()
  agent.onmessage = onAgent
  pty.onmessage = onPty
  return commands.startStreamingSpike(agent, pty)
}
