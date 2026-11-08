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
    status: 'inProgress',
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

  it('picks up a background agent reporting into a finished turn', () => {
    const builder = new TimelineBuilder()
    const events: AgentEvent[] = [
      user('go'),
      tool('agent', { kind: 'execute', name: 'Agent' }),
      end,
      user('next'),
      tool('child', { kind: 'execute', parent_id: 'agent' }),
      update('agent'),
    ]
    const entries = events.map((event, index) => ({
      seq: index + 1,
      atMs: index + 1,
      event,
    }))
    for (let count = 3; count <= entries.length; count += 1) {
      const next: MapTimelineInput = {
        events: entries.slice(0, count),
        approvals: [],
        findingsReady: false,
        showLive: false,
      }
      expect(builder.build(next)).toEqual(mapEventsToTimeline(next))
    }
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

describe('running subagents', () => {
  const agentCall = (id: string, extra: Partial<AgentEvent> = {}) =>
    tool(id, {
      kind: 'other',
      title: 'Agent',
      name: 'Agent',
      detail: 'Explore the auth flow',
      ...extra,
    } as Partial<AgentEvent>)

  const live = (events: AgentEvent[]) => {
    const items = mapEventsToTimeline({
      events: events.map((event, index) => ({
        seq: index + 1,
        atMs: index + 1,
        event,
      })),
      approvals: [],
      findingsReady: false,
      showLive: true,
    })
    const last = items.at(-1)
    return last?.kind === 'live' ? last.agents : []
  }

  it('lists an agent at work with the last thing it did', () => {
    const agents = live([
      user('go'),
      agentCall('a'),
      tool('c1', { parent_id: 'a', title: 'Read', detail: 'src/one.ts' }),
      tool('c2', { parent_id: 'a', title: 'Grep', detail: 'auth' }),
    ])
    expect(agents).toEqual([
      {
        id: 'a',
        description: 'Explore the auth flow',
        steps: 2,
        latest: 'Grep auth',
      },
    ])
  })

  it('drops an agent once it reports back', () => {
    expect(live([user('go'), agentCall('a'), update('a')])).toEqual([])
  })

  it('keeps a background agent listed after the turn that started it', () => {
    expect(live([user('go'), agentCall('a'), end, user('next')])).toHaveLength(
      1,
    )
  })

  it('forgets agents when the engine exits', () => {
    const exited = { type: 'engineExited', code: 1 } as unknown as AgentEvent
    expect(live([user('go'), agentCall('a'), exited])).toEqual([])
  })

  it('marks the agent step itself as running', () => {
    const items = mapEventsToTimeline({
      events: [user('go'), agentCall('a')].map((event, index) => ({
        seq: index + 1,
        event,
      })),
      approvals: [],
      findingsReady: false,
      showLive: false,
    })
    const run = items.find((item) => item.kind === 'toolRun')
    expect(run?.kind === 'toolRun' && run.steps[0]).toMatchObject({
      icon: 'agent',
      agent: true,
      status: 'inProgress',
      detail: 'Explore the auth flow',
    })
  })

  it('builds the same live row incrementally as from scratch', () => {
    const builder = new TimelineBuilder()
    const events: AgentEvent[] = [
      user('go'),
      agentCall('a'),
      end,
      user('next'),
      agent('meanwhile'),
      end,
      tool('child', { parent_id: 'a' }),
      update('a'),
    ]
    const entries = events.map((event, index) => ({
      seq: index + 1,
      atMs: index + 1,
      event,
    }))
    for (let count = 1; count <= entries.length; count += 1) {
      const next: MapTimelineInput = {
        events: entries.slice(0, count),
        approvals: [],
        findingsReady: false,
        showLive: true,
      }
      expect(builder.build(next)).toEqual(mapEventsToTimeline(next))
    }
  })
})
