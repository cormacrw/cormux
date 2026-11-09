import { describe, expect, it } from 'vitest'
import type { TimelineStep, TranscriptEvent } from '$lib/ipc/bindings'
import { currentConversation, heroLine } from './conversations'
import type { TimedEvent } from './timed-events'

let seq = 0
const at = (conversation: string, event: TranscriptEvent): TimedEvent => ({
  seq: ++seq,
  atMs: seq * 1000,
  conversation,
  event,
})
const ask = (conversation: string, text: string) =>
  at(conversation, { type: 'messageChunk', role: 'user', text })
const step = (conversation: string, step: TimelineStep) =>
  at(conversation, {
    type: 'step',
    id: `step-${seq}`,
    title: '',
    detail: null,
    step,
  })

describe('currentConversation', () => {
  it("is the newest event's conversation, without its session steps", () => {
    const events = [
      ask('first', 'Fix login'),
      step('after-2', { kind: 'sessionStarted', ended_session: 's1' }),
      ask('after-2', 'Write docs'),
      step('first', {
        kind: 'sessionResumed',
        conversation: 'first',
        ended_session: 's2',
      }),
      ask('first', 'And the logout too'),
    ]
    const current = currentConversation(events)
    expect(current.id).toBe('first')
    expect(
      current.events.map((row) =>
        row.event.type === 'messageChunk' ? row.event.text : row.event.type,
      ),
    ).toEqual(['Fix login', 'And the logout too'])
  })

  it('is empty right after a new session', () => {
    const events = [
      ask('first', 'Fix login'),
      step('after-2', { kind: 'sessionStarted', ended_session: null }),
    ]
    expect(currentConversation(events)).toEqual({ id: 'after-2', events: [] })
  })
})

describe('heroLine', () => {
  it('is stable for a seed', () => {
    expect(heroLine('th-1:first')).toBe(heroLine('th-1:first'))
  })
})
