import type { AgentEvent } from '$lib/ipc/bindings'

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
