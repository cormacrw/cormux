// Indexes built inside $derived are replaced whole, never mutated, so plain Map and Set are right.
/* eslint-disable svelte/prefer-svelte-reactivity */
import type { ScratchRow } from '$lib/ipc/bindings'
import { parseTimestampMs } from '$lib/format/relative-time'
import { reuseRows } from './reuse-rows'

/** A one-off conversation from before every thread had a worktree, kept to read. */
export type Scratch = {
  id: string
  title: string
  repoId: string
  threadId: string
  engine: string
  createdAtMs: number | null
}

export function scratchFromRow(row: ScratchRow): Scratch {
  return {
    id: row.id,
    title: row.title,
    repoId: row.repoId,
    threadId: row.threadId,
    engine: row.engine,
    createdAtMs: parseTimestampMs(row.createdAt),
  }
}

export class ScratchesStore {
  /** Newest first. */
  items = $state.raw<Scratch[]>([])

  private readonly byId = $derived(
    new Map(this.items.map((item) => [item.id, item])),
  )

  hydrate(items: Scratch[]) {
    this.items = reuseRows(this.items, items)
  }

  getById(id: string) {
    return this.byId.get(id)
  }
}

export const scratches = new ScratchesStore()
