/** Typing this word, then Space, Tab or Enter, turns the palette into a quick "add task" field. */
export const TODO_TRIGGER = 'todo'

export function isTodoTrigger(query: string): boolean {
  return query.trim().toLowerCase() === TODO_TRIGGER
}

/** A pasted "todo Buy milk" skips the key press: returns "Buy milk", or null when it isn't one. */
export function todoTitleFromQuery(query: string): string | null {
  const match = /^\s*todo\s+(\S[\s\S]*)$/i.exec(query)
  return match?.[1] ?? null
}
