// Indexes built inside $derived are replaced whole, never mutated, so plain Map and Set are right.
/* eslint-disable svelte/prefer-svelte-reactivity */
import type { ScratchRow, ThreadStatus } from '$lib/ipc/bindings'
import { parseTimestampMs } from '$lib/format/relative-time'
import { onSnapshot } from '$lib/core/sync'
import { reuseRows } from './reuse-rows'

/** A one-off conversation from before every thread had a worktree, kept to read. */
export type Scratch = {
  id: string
  title: string
  repoId: string
  threadId: string
  engine: string
  /** Its one thread's status. */
  status: ThreadStatus
  createdAtMs: number | null
}

function scratchFromRow(row: ScratchRow): Scratch {
  return {
    id: row.id,
    title: row.title,
    repoId: row.repoId,
    threadId: row.threadId,
    engine: row.engine,
    status: row.status,
    createdAtMs: parseTimestampMs(row.createdAt),
  }
}

export class ScratchesStore {
  /** Newest first. */
  items = $state.raw<Scratch[]>([])

  private readonly byId = $derived(
    new Map(this.items.map((item) => [item.id, item])),
  )
  private readonly byThread = $derived(
    new Map(this.items.map((item) => [item.threadId, item])),
  )

  hydrate(items: Scratch[]) {
    this.items = reuseRows(this.items, items)
  }

  getById(id: string) {
    return this.byId.get(id)
  }

  /** The scratch whose one thread this is. */
  forThread(threadId: string) {
    return this.byThread.get(threadId)
  }
}

export const scratches = new ScratchesStore()

onSnapshot((snapshot) =>
  scratches.hydrate(snapshot.persisted.scratches.map(scratchFromRow)),
)
