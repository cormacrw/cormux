import type { ScratchRow } from '$lib/ipc/bindings'
import { parseTimestampMs } from '$lib/homebase/relative-time'
import type { Thread } from './threads.svelte'

/** A titled one-off conversation against a repo. Internal code says scratch, never session. */
export type Scratch = {
  id: string
  title: string
  repoId: string
  threadId: string
  engine: string
  createdAtMs: number | null
}

export type ScratchStatus = 'needsAttention' | 'working' | 'paused' | 'idle'

/** First match wins: a pending approval, then running, then paused. */
export function scratchStatus(thread: Thread | undefined): ScratchStatus {
  if (!thread) return 'idle'
  if (thread.pendingApprovals > 0) return 'needsAttention'
  if (thread.status === 'running') return 'working'
  if (thread.status === 'paused' || thread.paused) return 'paused'
  return 'idle'
}

export const SCRATCH_STATUS_LABEL: Record<ScratchStatus, string> = {
  needsAttention: 'Needs attention',
  working: 'Working',
  paused: 'Paused',
  idle: 'Idle',
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
  /** Cards that already played their entrance animation. */
  seenIds = $state<string[]>([])

  hydrate(items: Scratch[]) {
    this.items = items
  }

  getById(id: string) {
    return this.items.find((item) => item.id === id)
  }

  markSeen(id: string) {
    if (this.seenIds.includes(id)) return
    this.seenIds = [...this.seenIds, id]
  }
}

export const scratches = new ScratchesStore()
