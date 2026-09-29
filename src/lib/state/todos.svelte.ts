import type { TodoRow } from '$lib/ipc/bindings'
import { commands, type CoreError } from '$lib/ipc'
import { coreErrorText } from '$lib/feedback/core-error'
import { showToast } from '$lib/feedback/show-toast'

export type Todo = TodoRow

function failed(fallback: string, error: CoreError) {
  showToast({
    tone: 'bad',
    parts: [{ type: 'text', value: coreErrorText(error, fallback) }],
  })
}

/** Title-only tasks. Changes apply locally first and roll back if the core refuses them. */
export class TodosStore {
  /** Oldest first. */
  items = $state<Todo[]>([])
  readonly pinned = $derived(this.items.filter((item) => item.pinned))

  hydrate(items: Todo[]) {
    this.items = items
  }

  async add(title: string) {
    const trimmed = title.trim()
    if (!trimmed) return false
    const result = await commands.createTodo(trimmed)
    if (result.status === 'error') {
      failed('Could not add the task', result.error)
      return false
    }
    this.items = [...this.items, result.data]
    return true
  }

  async remove(id: string) {
    const before = this.items
    this.items = before.filter((item) => item.id !== id)
    const result = await commands.deleteTodo(id)
    if (result.status === 'error') {
      this.items = before
      failed('Could not delete the task', result.error)
    }
  }

  async togglePin(id: string) {
    const todo = this.items.find((item) => item.id === id)
    if (!todo) return
    const pinned = !todo.pinned
    const patch = (value: boolean) => {
      this.items = this.items.map((item) =>
        item.id === id ? { ...item, pinned: value } : item,
      )
    }
    patch(pinned)
    const result = await commands.setTodoPinned(id, pinned)
    if (result.status === 'error') {
      patch(!pinned)
      failed('Could not update the task', result.error)
    }
  }
}

export const todos = new TodosStore()
