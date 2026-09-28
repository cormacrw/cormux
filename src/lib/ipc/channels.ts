import { Channel } from '@tauri-apps/api/core'
import {
  commands,
  type AgentChunk,
  type AgentEvent,
  type DiffUpdate,
  type PtyChunk,
} from './bindings'

/**
 * Subscribe helpers for high-volume ordered streams. The core owns the bytes;
 * the webview only listens while a surface is visible.
 */
export async function subscribeAgentChunks(
  threadId: string,
  onChunk: (chunk: AgentChunk) => void,
) {
  const channel = new Channel<AgentChunk>()
  channel.onmessage = onChunk
  return commands.subscribeAgentChunks(threadId, channel)
}

export async function subscribeAgentEvents(
  threadId: string,
  onEvent: (event: AgentEvent) => void,
) {
  const channel = new Channel<AgentEvent>()
  channel.onmessage = onEvent
  return commands.subscribeAgentEvents(threadId, channel)
}

export async function subscribePty(
  workspaceId: string,
  onChunk: (chunk: PtyChunk) => void,
) {
  const channel = new Channel<PtyChunk>()
  channel.onmessage = onChunk
  return commands.subscribePty(workspaceId, channel)
}

export async function subscribeDiffs(
  workspaceId: string,
  onUpdate: (update: DiffUpdate) => void,
) {
  const channel = new Channel<DiffUpdate>()
  channel.onmessage = onUpdate
  return commands.subscribeDiffs(workspaceId, channel)
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
