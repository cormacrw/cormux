import type { TodoRow } from '$lib/ipc/bindings'

/** `CMUX-12`: how prompts and the UI refer to a TODO, like a ClickUp task's ID. */
export function todoRef(todo: Pick<TodoRow, 'number'>): string {
  return `CMUX-${todo.number}`
}

/** The file the core keeps for a TODO, which an agent can read for its details. */
export function todoFile(todo: Pick<TodoRow, 'number'>, dir: string): string {
  return `${dir.replace(/\/+$/, '')}/${todoRef(todo)}.md`
}

/**
 * A TODO shaped like a ClickUp task for the composer's `#` picker and send-time
 * expansion: its ref as the ID, its title as the name, and its file as the link.
 */
export function todoAliasTask(todo: TodoRow, dir: string) {
  return {
    id: todo.id,
    customId: todoRef(todo),
    name: todo.title,
    url: todoFile(todo, dir),
    status: todo.doneAt ? 'done' : 'todo',
    kind: 'todo' as const,
  }
}
