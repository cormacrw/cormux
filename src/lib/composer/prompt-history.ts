import type { AgentEvent } from '$lib/ipc/bindings'

/** The user's prompts in a thread, oldest first. Back-to-back user chunks are one prompt. */
export function promptHistory(events: { event: AgentEvent }[]): string[] {
  const prompts: string[] = []
  let joining = false
  for (const { event } of events) {
    if (event.type === 'messageChunk' && event.role === 'user') {
      if (joining) prompts[prompts.length - 1] += event.text
      else prompts.push(event.text)
      joining = true
    } else {
      joining = false
    }
  }
  return prompts.filter((prompt) => prompt.trim().length > 0)
}

/**
 * Next history index for ↑ (older) or ↓ (newer); `null` means back to an empty composer.
 * Returns `undefined` when the key should do nothing.
 */
export function stepPromptHistory(
  index: number | null,
  length: number,
  direction: 'older' | 'newer',
): number | null | undefined {
  if (length === 0) return undefined
  if (direction === 'older') {
    if (index === null) return length - 1
    return index > 0 ? index - 1 : undefined
  }
  if (index === null) return undefined
  return index < length - 1 ? index + 1 : null
}
