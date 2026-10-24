import type { TodoRow } from '$lib/ipc/bindings'
import { commands, type CoreError } from '$lib/ipc'
import { coreErrorText } from '$lib/feedback/core-error'
import { showToast } from '$lib/feedback/show-toast'
import { sqliteNow } from '$lib/week/dates'

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

  private patch(id: string, patch: Partial<Todo>) {
    this.items = this.items.map((item) =>
      item.id === id ? { ...item, ...patch } : item,
    )
  }

  /** The Week page's details pane. A blank title is refused and leaves the task as it was. */
  async update(id: string, title: string, description: string) {
    const todo = this.items.find((item) => item.id === id)
    const trimmed = title.trim()
    if (!todo || !trimmed) return false
    if (todo.title === trimmed && todo.description === description) return true
    this.patch(id, { title: trimmed, description })
    const result = await commands.updateTodo(id, trimmed, description)
    if (result.status === 'error') {
      this.patch(id, { title: todo.title, description: todo.description })
      failed('Could not save the task', result.error)
      return false
    }
    return true
  }

  /** `plannedOn` is a local `YYYY-MM-DD` day, or null for the Week page's loading area. */
  async plan(id: string, plannedOn: string | null) {
    const todo = this.items.find((item) => item.id === id)
    if (!todo || todo.plannedOn === plannedOn) return
    this.patch(id, { plannedOn })
    const result = await commands.planTodo(id, plannedOn)
    if (result.status === 'error') {
      this.patch(id, { plannedOn: todo.plannedOn })
      failed('Could not plan the task', result.error)
    }
  }

  async setDone(id: string, done: boolean) {
    const todo = this.items.find((item) => item.id === id)
    if (!todo || (todo.doneAt != null) === done) return
    this.patch(id, { doneAt: done ? sqliteNow() : null })
    const result = await commands.setTodoDone(id, done)
    if (result.status === 'error') {
      this.patch(id, { doneAt: todo.doneAt })
      failed('Could not update the task', result.error)
      return
    }
    this.patch(id, { doneAt: result.data })
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
