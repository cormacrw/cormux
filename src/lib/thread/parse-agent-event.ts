import type { AgentEvent } from '$lib/ipc/bindings'
import { normalizeLegacyStep } from './legacy-steps'

export function parseAgentEventPayload(
  payload: string,
  seq?: number,
): AgentEvent | null {
  try {
    const raw = JSON.parse(payload) as Record<string, unknown>
    if (typeof raw.type === 'string') {
      return raw as AgentEvent
    }
    return normalizeLegacyStep(raw, seq)
  } catch {
    return null
  }
}
