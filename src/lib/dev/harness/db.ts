import type {
  ConversationSummary,
  SavedEvent,
  ThreadConversations,
  TranscriptEvent,
  TranscriptPage,
} from '$lib/ipc/bindings'
import {
  ATTACHED_LINE_PREFIX,
  FIRST_CONVERSATION,
  TRANSCRIPT_KINDS,
} from '$lib/ipc/bindings'
import { isSessionStep } from '$lib/thread/conversations'
import { createFixtures } from './fixtures'

/** The harness's stand-in for the core's store, one per browser session. */
export type HarnessDb = ReturnType<typeof createFixtures>

/** A `thread_events` row as the store keeps it. */
export type StoredEventRow = {
  id: number
  threadId: string
  seq: number
  kind: string
  payload: string
  /** SQLite `datetime('now')`: UTC, `YYYY-MM-DD HH:MM:SS`. */
  createdAt: string
}

function atMs(createdAt: string) {
  const ms = Date.parse(createdAt.replace(' ', 'T') + 'Z')
  return Number.isNaN(ms) ? 0 : ms
}

/** `created_at` as SQLite's `datetime('now')` writes it. */
export function storeNow() {
  return new Date().toISOString().slice(0, 19).replace('T', ' ')
}

/** A sent message as the core saves it, with any attachments named after it. */
export function shownMessage(text: string, names: string[]): string {
  if (!names.length) return text
  const line = `${ATTACHED_LINE_PREFIX}${names.join(', ')}`
  return text ? `${text}\n\n${line}` : line
}

function kindOf(event: TranscriptEvent): string {
  const kinds: Partial<Record<string, string>> = TRANSCRIPT_KINDS
  const kind = kinds[event.type]
  if (!kind) throw new Error(`the core never saves a ${event.type} event`)
  return kind
}

/** The next `seq` in a thread: the store numbers rows at write time. */
function nextSeq(db: HarnessDb, threadId: string) {
  let last = 0
  for (const row of db.timeline) {
    if (row.threadId === threadId) last = Math.max(last, row.seq)
  }
  return last + 1
}

export function appendEvent(
  db: HarnessDb,
  threadId: string,
  event: TranscriptEvent,
  createdAt = storeNow(),
): StoredEventRow {
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

function isUserMessage(row: StoredEventRow) {
  if (row.kind !== 'message') return false
  const event = JSON.parse(row.payload) as { role?: unknown }
  return event.role === 'user'
}

/** Each thread's latest turn end, engine exit or user message, as the core's `ACTIVITY_SQL`. */
export function threadActivity(db: HarnessDb) {
  const latest = new Map<string, StoredEventRow>()
  for (const row of db.timeline) {
    if (row.kind !== 'turn_end' && row.kind !== 'exit' && !isUserMessage(row))
      continue
    const seen = latest.get(row.threadId)
    if (!seen || row.seq > seen.seq) latest.set(row.threadId, row)
  }
  return [...latest.values()].map((row) => ({
    threadId: row.threadId,
    atMs: atMs(row.createdAt),
  }))
}

function threadRows(db: HarnessDb, threadId: string) {
  return db.timeline
    .filter((row) => row.threadId === threadId)
    .sort((a, b) => a.seq - b.seq)
}

function decode(row: StoredEventRow): TranscriptEvent | null {
  try {
    const event = JSON.parse(row.payload) as TranscriptEvent
    return typeof event.type === 'string' ? event : null
  } catch {
    return null
  }
}

/** Which conversation each row is in, by the core's rule (`transcript/conversations.rs`). */
function conversationsOf(rows: StoredEventRow[]) {
  const stretches: { from: number; id: string }[] = [
    { from: 0, id: FIRST_CONVERSATION },
  ]
  const ended = new Map<string, string>()
  const boundaries = new Set<number>()
  for (const row of rows) {
    const event = decode(row)
    if (event?.type !== 'step') continue
    const { step } = event
    if (!isSessionStep(step)) continue
    boundaries.add(row.seq)
    const current = stretches.at(-1)!.id
    if (step.ended_session) ended.set(current, step.ended_session)
    const target = step.kind === 'sessionResumed' ? step.conversation : null
    const known = target && stretches.some((stretch) => stretch.id === target)
    stretches.push({ from: row.seq, id: known ? target : `after-${row.seq}` })
  }
  const at = (seq: number) =>
    stretches.findLast((stretch) => stretch.from <= seq)!.id
  return {
    at,
    current: stretches.at(-1)!.id,
    known: (id: string) => stretches.some((stretch) => stretch.id === id),
    ended,
    boundaries,
  }
}

/**
 * One window of a transcript, as the core's `Transcript::window`: after `afterSeq` going
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
): TranscriptPage {
  const rows = threadRows(db, args.threadId)
  const limit = args.limit ?? rows.length
  const after = args.afterSeq
  const before = args.beforeSeq
  const window =
    after != null
      ? rows.filter((row) => row.seq > after).slice(0, limit)
      : rows.filter((row) => before == null || row.seq < before).slice(-limit)
  const conversations = conversationsOf(rows)
  const first = window[0]
  const last = window.at(-1)
  return {
    events: window.flatMap((row): SavedEvent[] => {
      const event = decode(row)
      return event
        ? [
            {
              seq: row.seq,
              atMs: atMs(row.createdAt),
              conversation: conversations.at(row.seq),
              event,
            },
          ]
        : []
    }),
    oldestSeq: first?.seq ?? null,
    newestSeq: last?.seq ?? null,
    hasOlder: first ? rows.some((row) => row.seq < first.seq) : false,
    hasNewer: last ? rows.some((row) => row.seq > last.seq) : false,
  }
}

export function currentConversation(db: HarnessDb, threadId: string) {
  return conversationsOf(threadRows(db, threadId)).current
}

/** The session conversation `id` resumes with, or undefined for one the thread doesn't have. */
export function sessionToResume(
  db: HarnessDb,
  threadId: string,
  id: string,
): string | null | undefined {
  const conversations = conversationsOf(threadRows(db, threadId))
  if (!conversations.known(id)) return undefined
  return conversations.ended.get(id) ?? null
}

/** As the core's `Transcript::conversations`. */
export function threadConversations(
  db: HarnessDb,
  threadId: string,
): ThreadConversations {
  const rows = threadRows(db, threadId)
  const conversations = conversationsOf(rows)
  const current = conversations.current
  const summaries = new Map<string, ConversationSummary>()
  let currentStartsAt: number | null = null
  for (const row of rows) {
    if (conversations.boundaries.has(row.seq)) continue
    const id = conversations.at(row.seq)
    if (id === current) currentStartsAt ??= row.seq
    const summary = summaries.get(id) ?? {
      id,
      firstPrompt: null,
      prompts: 0,
      lastAtMs: 0,
    }
    summaries.set(id, summary)
    summary.lastAtMs = atMs(row.createdAt)
    const event = decode(row)
    if (event?.type === 'messageChunk' && event.role === 'user') {
      summary.prompts += 1
      summary.firstPrompt ??= event.text.trim() || null
    }
  }
  return {
    current,
    currentStartsAt,
    earlier: [...summaries.values()]
      .filter((summary) => summary.id !== current && summary.prompts)
      .sort((a, b) => b.lastAtMs - a.lastAtMs),
  }
}

export function findThread(db: HarnessDb, threadId: string) {
  return db.snapshot.persisted.threads.find((row) => row.id === threadId)
}

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
