import { describe, expect, it } from 'vitest'
import type { TimelineStep, TranscriptEvent } from '$lib/ipc/bindings'
import { heroLine, splitConversations } from './conversations'

let seq = 0
const at = (event: TranscriptEvent) => ({ seq: ++seq, atMs: seq * 1000, event })
const ask = (text: string) => at({ type: 'messageChunk', role: 'user', text })
const reply = (text: string) =>
  at({ type: 'messageChunk', role: 'agent', text })
const step = (step: TimelineStep) =>
  at({ type: 'step', id: `step-${seq}`, title: '', detail: null, step })
const newSession = (ended: string | null) =>
  step({ kind: 'sessionStarted', ended_session: ended })

describe('splitConversations', () => {
  it('shows only what came after a new session', () => {
    const events = [ask('Fix login'), reply('Done'), newSession('sess-1')]
    const { current, earlier } = splitConversations(events)
    expect(current.events).toEqual([])
    expect(earlier).toHaveLength(1)
    expect(earlier[0]).toMatchObject({
      id: 'first',
      firstPrompt: 'Fix login',
      sessionId: 'sess-1',
      prompts: 1,
    })
  })

  it('switches back to a resumed conversation, which then grows', () => {
    const started = newSession('sess-1')
    const events = [
      ask('Fix login'),
      started,
      ask('Write docs'),
      step({
        kind: 'sessionResumed',
        conversation: 'first',
        ended_session: 'sess-2',
      }),
      ask('And the logout too'),
    ]
    const { current, earlier } = splitConversations(events)
    expect(current.id).toBe('first')
    expect(
      current.events.map((row) =>
        row.event.type === 'messageChunk' ? row.event.text : row.event.type,
      ),
    ).toEqual(['Fix login', 'And the logout too'])
    expect(earlier).toMatchObject([
      {
        id: `after-${started.seq}`,
        firstPrompt: 'Write docs',
        sessionId: 'sess-2',
      },
    ])
  })

  it('lists no empty conversations', () => {
    const events = [newSession(null), newSession(null)]
    expect(splitConversations(events).earlier).toEqual([])
  })
})

describe('heroLine', () => {
  it('is stable for a seed', () => {
    expect(heroLine('th-1:first')).toBe(heroLine('th-1:first'))
  })
})
