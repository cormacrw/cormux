import type { AgentEvent, ThreadEventRow } from '$lib/ipc/bindings'
import { eventCreatedAtMs } from '$lib/thread/thread-time'

function normalizeLegacyTool(raw: Record<string, unknown>): AgentEvent | null {
  if (typeof raw.title !== 'string') return null
  return {
    type: 'toolCall',
    id: `legacy-${raw.title}`,
    title: raw.title,
    name: null,
    kind: 'other',
    status: 'completed',
    locations: [],
    detail: typeof raw.detail === 'string' ? raw.detail : null,
  }
}

export function parseAgentEventPayload(payload: string): AgentEvent | null {
  try {
    const raw = JSON.parse(payload) as Record<string, unknown>
    if (typeof raw.type === 'string') {
      return raw as AgentEvent
    }
    return normalizeLegacyTool(raw)
  } catch {
    return null
  }
}

export function agentEventsForThread(
  rows: ThreadEventRow[],
  threadId: string,
): { seq: number; atMs: number; event: AgentEvent }[] {
  return rows
    .filter((row) => row.threadId === threadId)
    .sort((a, b) => a.seq - b.seq)
    .map((row) => {
      const event = parseAgentEventPayload(row.payload)
      if (!event) return null
      return { seq: row.seq, atMs: eventCreatedAtMs(row.createdAt), event }
    })
    .filter(
      (row): row is { seq: number; atMs: number; event: AgentEvent } =>
        row != null,
    )
}
