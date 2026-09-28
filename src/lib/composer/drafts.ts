export function applyDraftUpdate(
  prev: Record<string, string>,
  threadId: string,
  text: string,
): Record<string, string> {
  if (!text) {
    const next = { ...prev }
    delete next[threadId]
    return next
  }
  return { ...prev, [threadId]: text }
}

export function draftText(
  store: Record<string, string>,
  threadId: string,
): string {
  return store[threadId] ?? ''
}
