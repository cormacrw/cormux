// Indexes built inside $derived are replaced whole, never mutated, so a plain Map is right.
/* eslint-disable svelte/prefer-svelte-reactivity */
import type { EntityRow } from '$lib/ipc/bindings'
import { reuseRows } from './reuse-rows'

const NO_ENTITIES: EntityRow[] = []

/** What agents pinned to their threads with `attach_entity`, from the snapshot. */
export class EntitiesStore {
  items = $state.raw<EntityRow[]>([])

  private readonly byThread = $derived.by(() => {
    const map = new Map<string, EntityRow[]>()
    for (const row of this.items) {
      const list = map.get(row.threadId)
      if (list) list.push(row)
      else map.set(row.threadId, [row])
    }
    return map
  })

  hydrate(items: EntityRow[]) {
    this.items = reuseRows(this.items, items, (row) => String(row.id))
  }

  /** The thread's entities, oldest first; the same array until the list changes. */
  forThread(threadId: string): EntityRow[] {
    return this.byThread.get(threadId) ?? NO_ENTITIES
  }
}

export const entities = new EntitiesStore()
