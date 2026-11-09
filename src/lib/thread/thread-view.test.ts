import { describe, expect, it } from 'vitest'
import type { TranscriptEvent } from '$lib/ipc/bindings'
import type { TimedEvent } from './timed-events'
import {
  ThreadViewBuilder,
  type ThreadMeta,
  type ViewSources,
} from './thread-view'

const timed = (...events: TranscriptEvent[]): TimedEvent[] =>
  events.map((event, index) => ({
    seq: index + 1,
    atMs: index + 1,
    conversation: 'first',
    event,
  }))

const user = (text: string): TranscriptEvent => ({
  type: 'messageChunk',
  role: 'user',
  text,
})
const agent = (text: string): TranscriptEvent => ({
  type: 'messageChunk',
  role: 'agent',
  text,
})

const running: ThreadMeta = {
  status: 'running',
  paused: false,
  activity: 'Working',
  lead: true,
}

function sources(extra: Partial<ViewSources> = {}): ViewSources {
  return {
    events: timed(user('Fix it'), agent('On it')),
    approvals: [],
    meta: running,
    currentTool: null,
    hasOlder: false,
    generation: 0,
    ...extra,
  }
}

describe('a thread view', () => {
  it('is blank, not empty, until its history is read', () => {
    const view = new ThreadViewBuilder().view(sources({ events: undefined }))
    expect(view.loaded).toBe(false)
    expect(view.items).toEqual([])
  })

  it('says what the agent is doing while it works, apart from the timeline', () => {
    const view = new ThreadViewBuilder().view(
      sources({ currentTool: 'Reading src/a.ts' }),
    )
    expect(view.items.map((item) => item.kind)).toEqual(['user', 'reply'])
    expect(view.live).toEqual({
      title: 'Working',
      subtitle: 'Reading src/a.ts',
      paused: false,
      agents: [],
    })
    expect(view.activeIds).toEqual({
      reply: view.items[1]!.id,
      thought: null,
      run: null,
    })
  })

  it('has nothing live, and nothing still being written, once the agent is idle', () => {
    const view = new ThreadViewBuilder().view(
      sources({ meta: { ...running, status: 'idle' } }),
    )
    expect(view.live).toBeNull()
    expect(view.activeIds).toEqual({ reply: null, thought: null, run: null })
  })

  it('tells a paused agent and a worktree being set up apart', () => {
    const live = (
      meta: Partial<ThreadMeta>,
      currentTool: string | null = null,
    ) =>
      new ThreadViewBuilder().view(
        sources({ meta: { ...running, ...meta }, currentTool }),
      ).live
    expect(live({ status: 'paused', paused: true })).toMatchObject({
      title: 'Paused by you',
      subtitle: 'Resume to let the agent continue.',
      paused: true,
    })
    expect(live({ status: 'provisioning' })?.subtitle).toBe(
      'Setting up the worktree',
    )
    expect(live({ status: 'provisioning', lead: false })?.subtitle).toBe(
      'Joining the worktree',
    )
  })

  it('hands back the same view while nothing it shows changed', () => {
    const builder = new ThreadViewBuilder()
    const input = sources()
    const first = builder.view(input)
    expect(builder.view({ ...input })).toBe(first)
    const next = builder.view({ ...input, currentTool: 'Grep' })
    expect(next).not.toBe(first)
    expect(next.items).toBe(first.items)
  })

  it('offers your prompts back, oldest first', () => {
    const view = new ThreadViewBuilder().view(
      sources({ events: timed(user('one'), agent('a'), user('two')) }),
    )
    expect(view.promptHistory).toEqual(['one', 'two'])
  })
})
