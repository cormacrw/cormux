import { describe, expect, it } from 'vitest'
import type { ApprovalRow, ToolRole, TranscriptEvent } from '$lib/ipc/bindings'
import type { RunStep, TimelineItem } from '../timeline-types'
import { TimelineBuilder } from './builder'
import { stepText } from './step-text'

const command = (text: string): ToolRole => ({
  kind: 'command',
  command: text,
  description: null,
})

type Timed = { seq?: number; atMs?: number; event: TranscriptEvent }

/** The timeline the thread view builds for these events, seq'd in order unless given. */
function build(
  events: (TranscriptEvent | Timed)[],
  approvals: ApprovalRow[] = [],
) {
  return new TimelineBuilder().build({
    events: events.map((entry, index) =>
      'event' in entry
        ? { seq: entry.seq ?? index + 1, atMs: entry.atMs, event: entry.event }
        : { seq: index + 1, event: entry },
    ),
    approvals,
    findingsFor: null,
  })
}

function items(events: (TranscriptEvent | Timed)[], approvals?: ApprovalRow[]) {
  return build(events, approvals).items
}

function stepsOf(item: TimelineItem | undefined): RunStep[] {
  if (item?.kind !== 'toolRun') throw new Error(`not a run: ${item?.kind}`)
  return item.steps
}

const toolCall = (
  id: string,
  role: ToolRole,
  extra: Partial<Extract<TranscriptEvent, { type: 'toolCall' }>> = {},
): TranscriptEvent => ({
  type: 'toolCall',
  id,
  title: 'Tool',
  role,
  status: 'completed',
  locations: [],
  detail: null,
  ...extra,
})

const turnEnd: TranscriptEvent = {
  type: 'turnEnd',
  stop_reason: 'end_turn',
  error: null,
}

describe('the timeline fold', () => {
  it('shows tools the agent runs without asking, by command and path', () => {
    const shown = items([
      { type: 'messageChunk', role: 'thought', text: 'Check the repo.' },
      toolCall('toolu_1', command('ls -la'), {
        title: 'ls -la',
        status: 'inProgress',
        detail: 'List files',
      }),
      {
        type: 'toolCallUpdate',
        id: 'toolu_1',
        title: null,
        status: 'completed',
        locations: [],
      },
      toolCall(
        'toolu_2',
        { kind: 'read' },
        {
          title: 'Read',
          status: 'inProgress',
          locations: ['/repo/README.md'],
          detail: '/repo/README.md',
        },
      ),
    ])
    expect(shown[0]).toMatchObject({ kind: 'thought' })
    const steps = stepsOf(shown[1])
    expect(steps).toHaveLength(2)
    expect(steps[0]).toMatchObject({ status: 'completed' })
    expect(stepText(steps[0]!)).toMatchObject({
      title: 'Ran',
      detail: 'ls -la',
    })
    expect(stepText(steps[1]!)).toMatchObject({ detail: '/repo/README.md' })
  })

  it('coalesces user and agent message chunks', () => {
    expect(
      items([
        { type: 'messageChunk', role: 'user', text: 'Hello ' },
        { type: 'messageChunk', role: 'user', text: 'world' },
        { type: 'messageChunk', role: 'thought', text: 'Thinking…' },
      ]),
    ).toMatchObject([
      { kind: 'user', text: 'Hello world' },
      { kind: 'thought', text: 'Thinking…' },
    ])
  })

  it('marks steps that ran without asking', () => {
    const steps = stepsOf(
      items([
        {
          type: 'permission',
          id: 'perm',
          tool_call_id: 'read',
          title: 'Use Read',
          tool: 'Read',
          role: { kind: 'read' },
          detail: null,
          auto_approved: true,
        },
        toolCall('read', { kind: 'read' }),
        toolCall('next', { kind: 'read' }),
      ])[0],
    )
    expect(steps.map((step) => step.autoApproved)).toEqual([true, undefined])
  })

  it('fills an edit from its later update', () => {
    const edit = (id: string) =>
      toolCall(id, { kind: 'edit', verb: 'Edited' }, { status: 'pending' })
    const shown = items([
      edit('edit-1'),
      edit('edit-2'),
      {
        type: 'toolCallUpdate',
        id: 'edit-1',
        title: null,
        status: 'completed',
        locations: ['src/lib/auth.ts'],
      },
    ])
    expect(shown).toHaveLength(1)
    expect(stepsOf(shown[0])).toMatchObject([
      { id: 'edit-1', locations: ['src/lib/auth.ts'], status: 'completed' },
      { id: 'edit-2', locations: [], status: 'pending' },
    ])
  })

  it('keeps the end of what a test command printed', () => {
    const steps = stepsOf(
      items([
        toolCall('t', command('pnpm test'), { status: 'inProgress' }),
        {
          type: 'toolCallUpdate',
          id: 't',
          title: null,
          status: 'completed',
          locations: [],
          output: 'Tests: 2 passed',
        },
      ])[0],
    )
    expect(steps[0]).toMatchObject({ output: 'Tests: 2 passed' })
    expect(steps[0]?.kind === 'tool' && steps[0].run).toBeTruthy()
  })

  it('hides a thought when the reply repeats it', () => {
    const shown = items([
      {
        type: 'messageChunk',
        role: 'thought',
        text: 'The user wants to know what day it is.\n\nIt is Sunday.',
      },
      { type: 'messageChunk', role: 'agent', text: 'Sunday.' },
      turnEnd,
      { type: 'messageChunk', role: 'agent', text: 'next' },
    ])
    expect(shown).toMatchObject([
      { kind: 'reply', text: 'Sunday.' },
      { kind: 'reply', text: 'next' },
    ])
  })

  it('logs how long each turn took', () => {
    const start = 1_700_000_000_000
    const shown = items([
      {
        atMs: start,
        event: { type: 'messageChunk', role: 'user', text: 'Hi' },
      },
      {
        atMs: start + 1_000,
        event: { type: 'messageChunk', role: 'agent', text: 'Hey' },
      },
      { atMs: start + 125_000, event: turnEnd },
      { atMs: start + 126_000, event: { type: 'engineExited', code: 0 } },
      {
        atMs: start + 200_000,
        event: { type: 'messageChunk', role: 'user', text: 'Again' },
      },
      { atMs: start + 200_400, event: turnEnd },
    ])
    expect(shown.filter((item) => item.kind === 'worked')).toEqual([
      {
        kind: 'worked',
        id: 'run-worked-3',
        startedAtMs: start,
        seq: 3,
        atMs: start + 125_000,
      },
      {
        kind: 'worked',
        id: 'run-worked-6',
        startedAtMs: start + 200_000,
        seq: 6,
        atMs: start + 200_400,
      },
    ])
  })

  it('groups consecutive tool calls into one run', () => {
    const shown = items([
      toolCall('t1', { kind: 'read' }, { locations: ['src/a.ts'] }),
      toolCall('t2', { kind: 'search' }, { detail: 'pattern' }),
    ])
    expect(shown).toHaveLength(1)
    expect(stepsOf(shown[0])).toHaveLength(2)
  })

  it('maps permission rows using the stored approval payload', () => {
    const shown = items(
      [
        {
          type: 'permission',
          id: 'perm-1',
          tool_call_id: null,
          title: 'Use Bash',
          tool: 'Bash',
          role: command('pnpm test'),
          detail: 'pnpm test',
          auto_approved: false,
        },
      ],
      [
        {
          id: 'perm-1',
          threadId: 't1',
          status: 'pending',
          tool: 'execute',
          payload: {
            title: 'Run command',
            what: 'pnpm test',
            why: 'Bash',
            okLabel: 'Approve',
            noLabel: 'Deny',
          },
        },
      ],
    )
    expect(shown[0]).toMatchObject({
      kind: 'approval',
      title: 'Run command',
      okLabel: 'Approve',
      state: 'pending',
    })
  })

  it('shows a question only as its card, not as a hidden step', () => {
    const question = {
      question: 'Which colour?',
      header: 'Colour',
      options: [{ label: 'Red', description: 'Warm' }],
      multiSelect: false,
    }
    const shown = items(
      [
        toolCall(
          'toolu_ask',
          { kind: 'other', name: 'Ask user question' },
          { status: 'pending', hidden: true, parent_id: null },
        ),
        {
          type: 'permission',
          id: 'q1',
          tool_call_id: 'toolu_ask',
          title: 'Use Ask user question',
          tool: 'Ask user question',
          role: { kind: 'other', name: 'Ask user question' },
          detail: null,
          auto_approved: false,
        },
      ],
      [
        {
          id: 'q1',
          threadId: 't1',
          status: 'pending',
          tool: 'question',
          payload: {
            title: 'Question',
            what: 'Which colour?',
            why: '',
            okLabel: 'Send',
            noLabel: 'Skip',
            questions: [question],
          },
        },
      ],
    )
    expect(shown).toHaveLength(1)
    expect(shown[0]).toMatchObject({
      kind: 'approval',
      questions: [question],
      answers: {},
    })
  })

  it('shows a plan only as its approval, not as a hidden step', () => {
    const shown = items([
      toolCall('toolu_enter', { kind: 'planMode' }),
      toolCall(
        'toolu_plan',
        { kind: 'other', name: 'Exit plan mode' },
        { status: 'inProgress', hidden: true },
      ),
      {
        type: 'permission',
        id: 'plan-1',
        tool_call_id: 'toolu_plan',
        title: 'Use Exit plan mode',
        tool: 'Exit plan mode',
        role: { kind: 'other', name: 'Exit plan mode' },
        detail: null,
        auto_approved: false,
      },
    ])
    expect(shown.map((item) => item.kind)).toEqual(['toolRun', 'approval'])
    expect(stepText(stepsOf(shown[0])[0]!).title).toBe('Switched to plan mode')
  })

  it('ticks a turn’s plan off rather than adding another', () => {
    const plan = (status: string): TranscriptEvent =>
      ({
        type: 'plan',
        entries: [{ content: 'Step', status }],
      }) as TranscriptEvent
    const shown = items([
      { type: 'messageChunk', role: 'user', text: 'go' },
      plan('pending'),
      plan('completed'),
    ])
    expect(shown).toMatchObject([
      { kind: 'user' },
      { kind: 'plan', id: 'plan-2', statuses: ['completed'] },
    ])
  })

  it('ends the thread with the findings card once its review is ready', () => {
    const fold = new TimelineBuilder().build({
      events: [
        {
          seq: 7,
          event: { type: 'messageChunk', role: 'agent', text: 'Done' },
        },
      ],
      approvals: [],
      findingsFor: 'ws-1',
    })
    expect(fold.items.at(-1)).toEqual({
      kind: 'findings',
      id: 'findings',
      workspaceId: 'ws-1',
      seq: 7,
    })
  })

  it('leaves a hidden call out', () => {
    expect(
      items([
        toolCall(
          'toolu_1',
          { kind: 'other', name: 'Ask user question' },
          { status: 'pending', hidden: true },
        ),
      ]),
    ).toEqual([])
  })
})

describe('step words', () => {
  function textFor(role: ToolRole, detail: string | null, title = 'Tool') {
    return stepText(
      stepsOf(items([toolCall('t', role, { title, detail })])[0])[0]!,
    )
  }

  it('names an MCP call by its server, with its action before the detail', () => {
    expect(
      textFor({ kind: 'mcp', server: 'Trello', action: 'Read card' }, null),
    ).toMatchObject({ title: 'Used Trello', detail: 'Read card' })
    expect(
      textFor({ kind: 'mcp', server: 'Github', action: 'Get pr' }, 'cormux#3'),
    ).toMatchObject({ title: 'Used Github', detail: 'Get pr · cormux#3' })
  })

  it('names other tools by the title the engine gave them', () => {
    expect(
      textFor({ kind: 'other', name: 'Todo write' }, null, 'Todo write'),
    ).toMatchObject({ title: 'Todo write' })
  })

  it('shows a shell step as a short command preview', () => {
    const text =
      '/Users/me/.cursor/skills/impeccable comp-spec --comp mocks/01-pad-on-desk.png --grid'
    expect(textFor(command(text), null, text)).toEqual({
      title: 'Ran',
      detail: `${text.slice(0, 50).trimEnd()}…`,
      full: text,
    })
    expect(textFor(command(''), 'allowed by settings', 'Bash')).toMatchObject({
      title: 'Ran a command',
    })
  })

  it('shows a web fetch with a short URL', () => {
    expect(
      textFor(
        { kind: 'fetch', url: 'https://www.example.com/docs/page/' },
        'https://www.example.com/docs/page/',
      ),
    ).toEqual({
      title: 'Fetched',
      detail: 'example.com/docs/page',
      full: 'https://www.example.com/docs/page/',
    })
  })

  it('shows a skill and a tool lookup by what they name', () => {
    expect(
      textFor({ kind: 'skill', name: 'code-review' }, 'code-review'),
    ).toMatchObject({ title: 'Used skill', detail: 'code-review' })
    expect(textFor({ kind: 'toolLookup' }, 'select:Read')).toMatchObject({
      title: 'Looked up tools',
      detail: 'select:Read',
    })
  })
})

describe('item ids', () => {
  const events: TranscriptEvent[] = [
    { type: 'messageChunk', role: 'user', text: 'one' },
    { type: 'messageChunk', role: 'agent', text: 'first' },
    turnEnd,
    { type: 'messageChunk', role: 'user', text: 'two' },
    toolCall('toolu_9', { kind: 'read' }),
    { type: 'messageChunk', role: 'agent', text: 'second' },
  ]
  const rows = events.map((event, index) => ({
    seq: index + 1,
    atMs: 0,
    event,
  }))
  const input = { approvals: [], findingsFor: null }

  it('come from where the store saved each item and from tool call ids', () => {
    expect(items(events).map((item) => item.id)).toEqual([
      'user-1',
      'thought-2',
      'user-4',
      'run-toolu_9',
      'thought-6',
    ])
  })

  it('stay put when older events load in front', () => {
    const builder = new TimelineBuilder()
    const newest = builder.build({ ...input, events: rows.slice(3) }).items
    const all = builder.build({ ...input, events: rows }).items
    expect(all.slice(-newest.length).map((item) => item.id)).toEqual(
      newest.map((item) => item.id),
    )
  })

  it('stay unique for repeated branch switches and a step written twice', () => {
    const switched = (id: string, branch: string): TranscriptEvent => ({
      type: 'step',
      id,
      title: `Switched to \`${branch}\``,
      detail: null,
      step: {
        kind: 'branchSwitched',
        branch,
        change: { kind: 'checkedOut', previous: 'main' },
      },
    })
    const opened: TranscriptEvent = {
      type: 'step',
      id: 'pr-open-12',
      title: 'Opened PR #12',
      detail: null,
      step: { kind: 'prOpened', number: 12, branch: 'b', base: 'main' },
    }
    const ids = items([
      switched('step-1', 'feat/test'),
      switched('step-2', 'feat/colors'),
      switched('step-3', 'feat/test'),
      opened,
      opened,
    ]).flatMap((item) =>
      item.kind === 'toolRun'
        ? item.steps.map((step) => step.id)
        : item.kind === 'event'
          ? [item.id]
          : [],
    )
    expect(ids).toHaveLength(4)
    expect(new Set(ids).size).toBe(ids.length)
  })
})
