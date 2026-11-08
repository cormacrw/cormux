import type {
  ThreadEventPage,
  ThreadEventRow,
  TranscriptEvent,
} from '$lib/ipc/bindings'
import { createFixtures } from './fixtures'

/** The harness's stand-in for the core's store, one per browser session. */
export type HarnessDb = ReturnType<typeof createFixtures>

/** `created_at` as SQLite's `datetime('now')` writes it. */
export function storeNow() {
  return new Date().toISOString().slice(0, 19).replace('T', ' ')
}

/** A sent message as the core saves it, with any attachments named after it. */
export function shownMessage(text: string, names: string[]): string {
  if (!names.length) return text
  const line = `Attached: ${names.join(', ')}`
  return text ? `${text}\n\n${line}` : line
}

/** The `kind` column the core's transcript files an event under. */
function kindOf(event: TranscriptEvent): string {
  switch (event.type) {
    case 'step':
    case 'toolCall':
      return 'tool'
    case 'messageChunk':
    case 'briefing':
      return 'message'
    case 'sessionStarted':
      return 'session'
    case 'toolCallUpdate':
      return 'tool_update'
    case 'currentTool':
      return 'current_tool'
    case 'turnEnd':
      return 'turn_end'
    case 'engineExited':
      return 'exit'
    default:
      return event.type
  }
}

/** The next `seq` in a thread: the store numbers rows at write time. */
function nextSeq(db: HarnessDb, threadId: string) {
  let last = 0
  for (const row of db.timeline) {
    if (row.threadId === threadId) last = Math.max(last, row.seq)
  }
  return last + 1
}

/** Saves an event into a thread's transcript, numbered as the store would. */
export function appendEvent(
  db: HarnessDb,
  threadId: string,
  event: TranscriptEvent,
  createdAt = storeNow(),
): ThreadEventRow {
  const row = {
    id: Math.max(0, ...db.timeline.map((existing) => existing.id)) + 1,
    threadId,
    seq: nextSeq(db, threadId),
    kind: kindOf(event),
    payload: JSON.stringify(event),
    createdAt,
  }
  db.timeline.push(row)
  return row
}

function isUserMessage(row: ThreadEventRow) {
  if (row.kind !== 'message') return false
  const event = JSON.parse(row.payload) as { role?: unknown }
  return event.role === 'user'
}

/** Each thread's latest turn end, engine exit or user message, as the core's `ACTIVITY_SQL`. */
export function threadActivity(db: HarnessDb) {
  const latest = new Map<string, ThreadEventRow>()
  for (const row of db.timeline) {
    if (row.kind !== 'turn_end' && row.kind !== 'exit' && !isUserMessage(row))
      continue
    const seen = latest.get(row.threadId)
    if (!seen || row.seq > seen.seq) latest.set(row.threadId, row)
  }
  return [...latest.values()].map((row) => ({
    threadId: row.threadId,
    createdAt: row.createdAt,
  }))
}

/**
 * One window of a transcript, as the core's `thread_event_page`: after `afterSeq` going
 * forward, else the newest rows (before `beforeSeq`). The app always names its page size.
 */
export function eventPage(
  db: HarnessDb,
  args: {
    threadId: string
    afterSeq: number | null
    beforeSeq: number | null
    limit: number | null
  },
): ThreadEventPage {
  const rows = db.timeline
    .filter((row) => row.threadId === args.threadId)
    .sort((a, b) => a.seq - b.seq)
  const limit = args.limit ?? rows.length
  const after = args.afterSeq
  const before = args.beforeSeq
  const events =
    after != null
      ? rows.filter((row) => row.seq > after).slice(0, limit)
      : rows.filter((row) => before == null || row.seq < before).slice(-limit)
  const first = events[0]
  return {
    events,
    hasOlder: first ? rows.some((row) => row.seq < first.seq) : false,
  }
}

export function findThread(db: HarnessDb, threadId: string) {
  return db.snapshot.persisted.threads.find((row) => row.id === threadId)
}

/** A workspace's stored row and its snapshot record. */
export function findWorkspace(db: HarnessDb, workspaceId: string) {
  return {
    row: db.snapshot.persisted.workspaces.find(
      (workspace) => workspace.id === workspaceId,
    ),
    record: db.snapshot.workspaces.find(
      (workspace) => workspace.id === workspaceId,
    ),
  }
}
