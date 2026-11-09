import { describe, expect, it } from 'vitest'
import type { ThreadConversations } from '$lib/ipc/bindings'
import transcriptCases from '$lib/ipc/rule-cases/transcript.json?raw'
import {
  eventPage,
  sessionToResume,
  threadActivity,
  threadConversations,
  type StoredEventRow,
} from './db'
import { createFixtures } from './fixtures'

/** The cases the core's `transcript/rule_cases.rs` checks its own rules against. */
const cases = JSON.parse(transcriptCases) as {
  rows: Omit<StoredEventRow, 'id'>[]
  activity: { threadId: string; atMs: number }[]
  conversationAt: Record<string, Record<string, string>>
  conversations: Record<string, ThreadConversations>
  resume: { threadId: string; id: string; session: string | null }[]
  unknownConversations: string[]
}

function seeded() {
  const db = createFixtures()
  db.timeline = cases.rows.map((row, index) => ({ id: index + 1, ...row }))
  return db
}

describe('the harness reads a transcript as the core does', () => {
  it('times each thread by its last turn end or user message', () => {
    const activity = threadActivity(seeded()).sort((a, b) =>
      a.threadId.localeCompare(b.threadId),
    )
    expect(activity).toEqual(cases.activity)
  })

  it('puts each row in the conversation the core does', () => {
    const db = seeded()
    for (const [threadId, expected] of Object.entries(cases.conversationAt)) {
      const page = eventPage(db, {
        threadId,
        afterSeq: null,
        beforeSeq: null,
        limit: null,
      })
      const at = Object.fromEntries(
        page.events.map((event) => [String(event.seq), event.conversation]),
      )
      expect(at).toEqual(expected)
      expect(threadConversations(db, threadId)).toEqual(
        cases.conversations[threadId],
      )
    }
  })

  it('resumes each conversation with the session that ended it', () => {
    const db = seeded()
    for (const { threadId, id, session } of cases.resume) {
      expect(sessionToResume(db, threadId, id)).toBe(session)
    }
    for (const id of cases.unknownConversations) {
      expect(sessionToResume(db, 't1', id)).toBeUndefined()
    }
  })
})
