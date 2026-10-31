import type { ScratchRow } from '$lib/ipc/bindings'
import { parseTimestampMs } from '$lib/homebase/relative-time'

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
  items = $state<Scratch[]>([])

  hydrate(items: Scratch[]) {
    this.items = items
  }

  getById(id: string) {
    return this.items.find((item) => item.id === id)
  }
}

export const scratches = new ScratchesStore()
