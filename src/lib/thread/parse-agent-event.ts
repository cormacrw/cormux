import type { AgentEvent, ThreadEventRow } from '$lib/ipc/bindings'
import { eventCreatedAtMs } from '$lib/thread/thread-time'

function normalizeLegacyTool(
  raw: Record<string, unknown>,
  seq: number | undefined,
): AgentEvent | null {
  if (typeof raw.title !== 'string') return null
  return {
    type: 'toolCall',
    // Titles repeat (switching back to a branch), and a repeated key crashes the timeline.
    id: seq === undefined ? `legacy-${raw.title}` : `legacy-${seq}`,
    title: raw.title,
    name: null,
    kind: 'other',
    status: 'completed',
    locations: [],
    detail: typeof raw.detail === 'string' ? raw.detail : null,
  }
}

export function parseAgentEventPayload(
  payload: string,
  seq?: number,
): AgentEvent | null {
  try {
    const raw = JSON.parse(payload) as Record<string, unknown>
    if (typeof raw.type === 'string') {
      return raw as AgentEvent
    }
    return normalizeLegacyTool(raw, seq)
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
      const event = parseAgentEventPayload(row.payload, row.seq)
      if (!event) return null
      return { seq: row.seq, atMs: eventCreatedAtMs(row.createdAt), event }
    })
    .filter(
      (row): row is { seq: number; atMs: number; event: AgentEvent } =>
        row != null,
    )
}
