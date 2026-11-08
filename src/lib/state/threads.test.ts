import { describe, expect, it, vi } from 'vitest'
import type { TranscriptEvent } from '$lib/ipc/bindings'

vi.mock('$lib/core/sync', () => ({ onSnapshot: () => () => {} }))

const { threadsFromSnapshot } = await import('./threads.svelte')
const { createFixtures, LEAD_THREAD_ID } =
  await import('$lib/dev/harness/fixtures')
const { appendEvent, findThread } = await import('$lib/dev/harness/db')

const resumed: TranscriptEvent = {
  type: 'step',
  id: 'step-resumed',
  title: 'Resumed an earlier conversation',
  detail: null,
  step: {
    kind: 'sessionResumed',
    conversation: 'conv-1',
    ended_session: 'session-2',
  },
}

describe('threadsFromSnapshot', () => {
  it('has no session to clear after resuming a conversation that had none', () => {
    const db = createFixtures()
    appendEvent(db, LEAD_THREAD_ID, resumed)
    findThread(db, LEAD_THREAD_ID)!.sessionId = null

    const lead = threadsFromSnapshot(db.snapshot).find(
      (thread) => thread.id === LEAD_THREAD_ID,
    )

    expect(lead?.hasSession).toBe(false)
  })

  it('has a session to clear once the engine started one', () => {
    const db = createFixtures()
    findThread(db, LEAD_THREAD_ID)!.sessionId = 'session-1'

    const lead = threadsFromSnapshot(db.snapshot).find(
      (thread) => thread.id === LEAD_THREAD_ID,
    )

    expect(lead?.hasSession).toBe(true)
  })
})
