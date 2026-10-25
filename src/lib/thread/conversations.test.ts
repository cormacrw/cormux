import { describe, expect, it } from 'vitest'
import type { AgentEvent } from '$lib/ipc/bindings'
import { heroLine, RESUME_TITLE, splitConversations } from './conversations'
import { NEW_SESSION_TITLE } from './new-session'

let seq = 0
const at = (event: AgentEvent) => ({ seq: ++seq, atMs: seq * 1000, event })
const ask = (text: string) => at({ type: 'messageChunk', role: 'user', text })
const reply = (text: string) =>
  at({ type: 'messageChunk', role: 'agent', text })
const step = (title: string, detail: object) =>
  at({
    type: 'toolCall',
    id: `control-${seq}`,
    title,
    name: null,
    kind: 'other',
    status: 'completed',
    locations: [],
    detail: JSON.stringify(detail),
  })

describe('splitConversations', () => {
  it('shows only what came after a new session', () => {
    const events = [
      ask('Fix login'),
      reply('Done'),
      step(NEW_SESSION_TITLE, { endedSession: 'sess-1' }),
    ]
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
    const newSession = step(NEW_SESSION_TITLE, { endedSession: 'sess-1' })
    const events = [
      ask('Fix login'),
      newSession,
      ask('Write docs'),
      step(RESUME_TITLE, { conversation: 'first', endedSession: 'sess-2' }),
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
        id: `after-${newSession.seq}`,
        firstPrompt: 'Write docs',
        sessionId: 'sess-2',
      },
    ])
  })

  it('lists no empty conversations', () => {
    const events = [
      step(NEW_SESSION_TITLE, { endedSession: null }),
      step(NEW_SESSION_TITLE, { endedSession: null }),
    ]
    expect(splitConversations(events).earlier).toEqual([])
  })
})

describe('heroLine', () => {
  it('is stable for a seed', () => {
    expect(heroLine('th-1:first')).toBe(heroLine('th-1:first'))
  })
})
