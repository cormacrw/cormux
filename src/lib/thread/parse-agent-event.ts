import type { TranscriptEvent } from '$lib/ipc/bindings'

/** A saved row's event. Every row is typed (the core rewrote older ones once), so one without a type is something this build can't show. */
export function parseAgentEventPayload(
  payload: string,
): TranscriptEvent | null {
  try {
    const raw = JSON.parse(payload) as Record<string, unknown>
    return typeof raw.type === 'string' ? (raw as TranscriptEvent) : null
  } catch {
    return null
  }
}
