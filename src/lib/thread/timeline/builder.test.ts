import { describe, expect, it } from 'vitest'
import type { TranscriptEvent } from '$lib/ipc/bindings'
import { TimelineBuilder } from './builder'
import { foldTimeline, type FoldInput } from './fold'

type ToolCall = Extract<TranscriptEvent, { type: 'toolCall' }>

const tool = (id: string, extra: Partial<ToolCall> = {}): TranscriptEvent =>
  ({
    type: 'toolCall',
    id,
    title: 'Read',
    role: { kind: 'read' },
    status: 'inProgress',
    locations: [],
    detail: null,
    ...extra,
  }) as TranscriptEvent

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
const thought = (text: string): TranscriptEvent => ({
  type: 'messageChunk',
  role: 'thought',
  text,
})
const end: TranscriptEvent = {
  type: 'turnEnd',
  stop_reason: 'end',
  error: null,
}
const plan = (status: string): TranscriptEvent =>
  ({
    type: 'plan',
    entries: [{ content: 'Step', status }],
  }) as TranscriptEvent
const update = (id: string): TranscriptEvent =>
  ({
    type: 'toolCallUpdate',
    id,
    title: 'Late title',
    status: 'completed',
    locations: [],
    output: 'Tests: 2 passed',
  }) as TranscriptEvent

const SCRIPT: TranscriptEvent[] = [
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
  { type: 'usage', used_tokens: 10, context_size: 100 } as TranscriptEvent,
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

function input(count: number): FoldInput {
  return {
    events: rows.slice(0, count),
    approvals: [],
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
      expect(builder.build(input(count))).toEqual(foldTimeline(input(count)))
    }
  })

  it('keeps the objects of rows that did not change', () => {
    const builder = new TimelineBuilder()
    const before = builder.build(input(4)).items
    const after = builder.build(input(5)).items
    expect(after[0]).toBe(before[0])
    expect(after.find((item) => item.kind === 'thought')).toBe(
      before.find((item) => item.kind === 'thought'),
    )
  })

  it('picks up a background agent reporting into a finished turn', () => {
    const builder = new TimelineBuilder()
    const events: TranscriptEvent[] = [
      user('go'),
      tool('agent', { role: { kind: 'subagent', description: null } }),
      end,
      user('next'),
      tool('child', {
        role: { kind: 'command', command: 'ls', description: null },
        parent_id: 'agent',
      }),
      update('agent'),
    ]
    const entries = events.map((event, index) => ({
      seq: index + 1,
      atMs: index + 1,
      event,
    }))
    for (let count = 3; count <= entries.length; count += 1) {
      const next: FoldInput = {
        events: entries.slice(0, count),
        approvals: [],
      }
      expect(builder.build(next)).toEqual(foldTimeline(next))
    }
  })

  it('rebuilds when an approval changes', () => {
    const permission = {
      type: 'permission',
      id: 'p',
      tool_call_id: null,
      title: 'Run',
      tool: 'Bash',
      role: { kind: 'command', command: 'ls', description: null },
      detail: null,
      auto_approved: false,
    } as TranscriptEvent
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
    const base = { events }
    const builder = new TimelineBuilder()
    builder.build({ ...base, approvals: [row('pending')] as never })
    const items = builder.build({
      ...base,
      approvals: [row('approved')] as never,
    })
    expect(items.items[0]).toMatchObject({
      kind: 'approval',
      state: 'approved',
    })
  })
})

describe('running subagents', () => {
  const agentCall = (id: string) =>
    tool(id, {
      title: 'Agent',
      role: { kind: 'subagent', description: 'Explore the auth flow' },
      detail: 'Explore the auth flow',
    })

  const live = (events: TranscriptEvent[]) => {
    return foldTimeline({
      events: events.map((event, index) => ({
        seq: index + 1,
        atMs: index + 1,
        event,
      })),
      approvals: [],
    }).agents
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
    const exited = {
      type: 'engineExited',
      code: 1,
    } as unknown as TranscriptEvent
    expect(live([user('go'), agentCall('a'), exited])).toEqual([])
  })

  it('marks the agent step itself as running', () => {
    const { items } = foldTimeline({
      events: [user('go'), agentCall('a')].map((event, index) => ({
        seq: index + 1,
        event,
      })),
      approvals: [],
    })
    const run = items.find((item) => item.kind === 'toolRun')
    expect(run?.kind === 'toolRun' && run.steps[0]).toMatchObject({
      role: { kind: 'subagent' },
      status: 'inProgress',
      detail: 'Explore the auth flow',
    })
  })

  it('builds the same live row incrementally as from scratch', () => {
    const builder = new TimelineBuilder()
    const events: TranscriptEvent[] = [
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
      const next: FoldInput = {
        events: entries.slice(0, count),
        approvals: [],
      }
      expect(builder.build(next)).toEqual(foldTimeline(next))
    }
  })
})
