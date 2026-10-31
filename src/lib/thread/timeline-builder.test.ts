import { describe, expect, it } from 'vitest'
import type { AgentEvent } from '$lib/ipc/bindings'
import {
  TimelineBuilder,
  mapEventsToTimeline,
  type MapTimelineInput,
} from './map-events-to-timeline'

const tool = (id: string, extra: Partial<AgentEvent> = {}): AgentEvent =>
  ({
    type: 'toolCall',
    id,
    title: 'Read',
    name: null,
    kind: 'read',
    status: 'in_progress',
    locations: [],
    detail: null,
    ...extra,
  }) as AgentEvent

const user = (text: string): AgentEvent => ({
  type: 'messageChunk',
  role: 'user',
  text,
})
const agent = (text: string): AgentEvent => ({
  type: 'messageChunk',
  role: 'agent',
  text,
})
const thought = (text: string): AgentEvent => ({
  type: 'messageChunk',
  role: 'thought',
  text,
})
const end: AgentEvent = { type: 'turnEnd', stop_reason: 'end', error: null }
const plan = (status: string): AgentEvent =>
  ({
    type: 'plan',
    entries: [{ content: 'Step', status }],
  }) as AgentEvent
const update = (id: string): AgentEvent =>
  ({
    type: 'toolCallUpdate',
    id,
    title: 'Late title',
    kind: null,
    status: 'completed',
    locations: [],
    output: 'Tests: 2 passed',
  }) as AgentEvent

const SCRIPT: AgentEvent[] = [
  user('one'),
  thought('thinking'),
  agent('rep'),
  agent('ly'),
  tool('a'),
  update('a'),
  end,
  thought('a thought just after a turn'),
  plan('pending'),
  end,
  plan('completed'),
  user('two'),
  { type: 'usage', used_tokens: 10, context_size: 100 } as AgentEvent,
  tool('b'),
  tool('c', { parent_id: 'b' }),
  end,
  // Reaches back into a finished turn.
  update('b'),
  tool('d', { parent_id: 'a' }),
  agent('late'),
  end,
  agent('after'),
]

function input(count: number): MapTimelineInput {
  return {
    events: rows.slice(0, count),
    approvals: [],
    findingsReady: false,
    showLive: count % 2 === 0,
  }
}

const rows = SCRIPT.map((event, index) => ({
  seq: index + 1,
  atMs: (index + 1) * 1000,
  event,
}))

describe('TimelineBuilder', () => {
  it('matches a full rebuild after every event', () => {
    const builder = new TimelineBuilder()
    for (let count = 0; count <= rows.length; count += 1) {
      expect(builder.build(input(count))).toEqual(
        mapEventsToTimeline(input(count)),
      )
    }
  })

  it('keeps the objects of rows that did not change', () => {
    const builder = new TimelineBuilder()
    const before = builder.build(input(4))
    const after = builder.build(input(5))
    expect(after[0]).toBe(before[0])
    expect(after.find((item) => item.kind === 'thought')).toBe(
      before.find((item) => item.kind === 'thought'),
    )
  })

  it('rebuilds when an approval changes', () => {
    const permission = {
      type: 'permission',
      id: 'p',
      tool_call_id: null,
      title: 'Run',
      tool_name: 'Bash',
      kind: 'execute',
      detail: null,
      auto_approved: false,
    } as AgentEvent
    const events = [permission, end].map((event, index) => ({
      seq: index + 1,
      atMs: 1,
      event,
    }))
    const row = (status: string) => ({
      id: 'p',
      threadId: 't',
      status,
      payload: '',
    })
    const base = { events, findingsReady: false, showLive: false }
    const builder = new TimelineBuilder()
    builder.build({ ...base, approvals: [row('pending')] as never })
    const items = builder.build({
      ...base,
      approvals: [row('approved')] as never,
    })
    expect(items[0]).toMatchObject({ kind: 'approval', state: 'approved' })
  })
})
