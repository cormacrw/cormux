import type { AgentEvent } from '$lib/ipc/bindings'
import type { ThreadEventRow } from '$lib/ipc/bindings'

export function parseAgentEventPayload(payload: string): AgentEvent | null {
  try {
    return JSON.parse(payload) as AgentEvent
  } catch {
    return null
  }
}

export function agentEventsForThread(
  rows: ThreadEventRow[],
  threadId: string,
): { seq: number; event: AgentEvent }[] {
  return rows
    .filter((row) => row.threadId === threadId)
    .sort((a, b) => a.seq - b.seq)
    .map((row) => {
      const event = parseAgentEventPayload(row.payload)
      if (!event) return null
      return { seq: row.seq, event }
    })
    .filter((row): row is { seq: number; event: AgentEvent } => row != null)
}
