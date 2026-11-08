import { describe, expect, it } from 'vitest'
import type { ToolRole, TranscriptEvent } from '$lib/ipc/bindings'
import {
  TimelineBuilder,
  mapEventsToTimeline,
  shouldShowLiveRow,
} from './map-events-to-timeline'

const command = (text: string): ToolRole => ({
  kind: 'command',
  command: text,
  description: null,
})

describe('mapEventsToTimeline', () => {
  it('shows tools Claude runs without asking, by command and path', () => {
    const items = mapEventsToTimeline({
      events: [
        {
          seq: 1,
          event: {
            type: 'messageChunk',
            role: 'thought',
            text: 'Check the repo.',
          },
        },
        {
          seq: 2,
          event: {
            type: 'toolCall',
            id: 'toolu_1',
            title: 'ls -la',
            role: command('ls -la'),
            status: 'inProgress',
            locations: [],
            detail: 'List files',
          },
        },
        {
          seq: 3,
          event: {
            type: 'toolCallUpdate',
            id: 'toolu_1',
            title: null,
            status: 'completed',
            locations: [],
          },
        },
        {
          seq: 4,
          event: {
            type: 'toolCall',
            id: 'toolu_2',
            title: 'Read',
            role: { kind: 'read' },
            status: 'inProgress',
            locations: ['/repo/README.md'],
            detail: '/repo/README.md',
          },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
    })
    expect(items[0]).toMatchObject({ kind: 'thought', role: 'thought' })
    const steps = items[1]?.kind === 'toolRun' ? items[1].steps : []
    expect(steps).toHaveLength(2)
    expect(JSON.stringify(steps[0])).toContain('ls -la')
    expect(steps[1]).toMatchObject({ kind: 'tool', detail: '/repo/README.md' })
  })

  it('coalesces user and agent message chunks', () => {
    const items = mapEventsToTimeline({
      events: [
        {
          seq: 1,
          event: { type: 'messageChunk', role: 'user', text: 'Hello ' },
        },
        {
          seq: 2,
          event: { type: 'messageChunk', role: 'user', text: 'world' },
        },
        {
          seq: 3,
          event: { type: 'messageChunk', role: 'thought', text: 'Thinking…' },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
    })
    expect(items).toHaveLength(2)
    expect(items[0]).toMatchObject({ kind: 'user', text: 'Hello world' })
    expect(items[1]).toMatchObject({
      kind: 'thought',
      role: 'thought',
      text: 'Thinking…',
    })
  })

  it('marks scratch reads read-only with no worktree', () => {
    const items = mapEventsToTimeline({
      events: [
        {
          seq: 1,
          event: {
            type: 'toolCall',
            id: 'open',
            title: 'Opened my-app',
            role: { kind: 'read' },
            status: 'completed',
            locations: [],
            detail: '~/code/my-app',
          },
        },
        {
          seq: 2,
          event: {
            type: 'toolCall',
            id: 'bash',
            title: 'git log',
            role: command('git log'),
            status: 'completed',
            locations: [],
            detail: null,
          },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
      scratch: true,
    })
    expect(items[0]).toMatchObject({ kind: 'toolRun' })
    const steps = items[0]?.kind === 'toolRun' ? items[0].steps : []
    expect(steps[0]).toMatchObject({
      title: 'Opened my-app',
      detail: '~/code/my-app',
      chips: [{ label: 'Read-only, no worktree' }],
    })
    expect(steps[1]).toMatchObject({
      title: 'Ran',
      detail: 'git log',
      quiet: true,
      chips: undefined,
    })
  })

  it('shows a shell step as a short command preview', () => {
    const text =
      '/Users/me/.cursor/skills/impeccable comp-spec --comp mocks/01-pad-on-desk.png --grid'
    const items = mapEventsToTimeline({
      events: [
        {
          seq: 1,
          event: {
            type: 'toolCall',
            id: 'cursor-shell',
            title: text,
            role: command(text),
            status: 'completed',
            locations: [],
            detail: null,
          },
        },
        {
          seq: 2,
          event: {
            type: 'toolCall',
            id: 'claude-bash',
            title: 'Bash',
            role: command(''),
            status: 'completed',
            locations: [],
            detail: 'allowed by settings',
          },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
    })
    const steps = items[0]?.kind === 'toolRun' ? items[0].steps : []
    expect(steps[0]).toMatchObject({
      title: 'Ran',
      detail: `${text.slice(0, 50).trimEnd()}…`,
      rawDetail: text,
      quiet: true,
    })
    expect(steps[1]).toMatchObject({ title: 'Ran a command', quiet: true })
  })

  it('fills an edit from its later update', () => {
    const editCall = (id: string) => ({
      type: 'toolCall' as const,
      id,
      title: 'Edit File',
      role: { kind: 'edit', verb: 'Edited' } as const,
      status: 'pending' as const,
      locations: [],
      detail: null,
    })
    const items = mapEventsToTimeline({
      events: [
        { seq: 1, event: editCall('edit-1') },
        { seq: 2, event: editCall('edit-2') },
        {
          seq: 3,
          event: {
            type: 'toolCallUpdate',
            id: 'edit-1',
            title: null,
            status: 'completed',
            locations: ['src/lib/auth.ts'],
          },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
    })
    expect(items).toHaveLength(1)
    const steps = items[0]?.kind === 'toolRun' ? items[0].steps : []
    expect(steps).toMatchObject([
      { kind: 'edit', id: 'edit-1', path: 'src/lib/auth.ts', verb: 'Edited' },
      { kind: 'edit', id: 'edit-2', path: '', verb: 'Edited' },
    ])
  })

  it('hides a thought when the reply repeats it', () => {
    const items = mapEventsToTimeline({
      events: [
        {
          seq: 1,
          event: {
            type: 'messageChunk',
            role: 'thought',
            text: 'The user wants to know what day it is.\n\nIt is Sunday, September 27, 2026.',
          },
        },
        {
          seq: 2,
          event: {
            type: 'messageChunk',
            role: 'agent',
            text: 'Sunday, September 27, 2026.',
          },
        },
        {
          seq: 3,
          event: { type: 'turnEnd', stop_reason: 'end_turn', error: null },
        },
        {
          seq: 4,
          event: { type: 'messageChunk', role: 'agent', text: 'next' },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
    })
    expect(
      items.map((item) => (item.kind === 'thought' ? item.text : item.kind)),
    ).toEqual(['Sunday, September 27, 2026.', 'next'])
  })

  it('logs how long each turn took', () => {
    const start = 1_700_000_000_000
    const items = mapEventsToTimeline({
      events: [
        {
          seq: 1,
          atMs: start,
          event: { type: 'messageChunk', role: 'user', text: 'Hi' },
        },
        {
          seq: 2,
          atMs: start + 1_000,
          event: { type: 'messageChunk', role: 'agent', text: 'Hey' },
        },
        {
          seq: 3,
          atMs: start + 125_000,
          event: { type: 'turnEnd', stop_reason: 'end_turn', error: null },
        },
        {
          seq: 4,
          atMs: start + 126_000,
          event: { type: 'engineExited', code: 0 },
        },
        {
          seq: 5,
          atMs: start + 200_000,
          event: { type: 'messageChunk', role: 'user', text: 'Again' },
        },
        {
          seq: 6,
          atMs: start + 200_400,
          event: { type: 'turnEnd', stop_reason: 'end_turn', error: null },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
    })
    expect(
      items.flatMap((item) =>
        item.kind === 'toolRun'
          ? item.steps.flatMap((step) =>
              step.kind === 'tool' && step.quiet ? [step.title] : [],
            )
          : [],
      ),
    ).toEqual(['Worked for 2m 5s', 'Worked for 1s'])
  })

  it('groups consecutive tool calls into one run', () => {
    const items = mapEventsToTimeline({
      events: [
        {
          seq: 1,
          event: {
            type: 'toolCall',
            id: 't1',
            title: 'Read files',
            role: { kind: 'read' },
            status: 'completed',
            locations: ['src/a.ts'],
            detail: null,
          },
        },
        {
          seq: 2,
          event: {
            type: 'toolCall',
            id: 't2',
            title: 'Grep',
            role: { kind: 'search' },
            status: 'completed',
            locations: [],
            detail: 'pattern',
          },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
    })
    expect(items).toHaveLength(1)
    const run = items[0]
    expect(run?.kind).toBe('toolRun')
    if (run?.kind === 'toolRun') {
      expect(run.steps).toHaveLength(2)
    }
  })

  it('maps edit tool calls to edit steps', () => {
    const items = mapEventsToTimeline({
      events: [
        {
          seq: 1,
          event: {
            type: 'toolCall',
            id: 'e1',
            title: 'Write',
            role: { kind: 'edit', verb: 'Edited' },
            status: 'completed',
            locations: ['src/lib/auth.ts'],
            detail: null,
          },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
    })
    const run = items[0]
    expect(run?.kind).toBe('toolRun')
    if (run?.kind === 'toolRun') {
      expect(run.steps[0]).toMatchObject({
        kind: 'edit',
        path: 'src/lib/auth.ts',
      })
    }
  })

  it('maps permission rows using stored approval payload', () => {
    const items = mapEventsToTimeline({
      events: [
        {
          seq: 1,
          event: {
            type: 'permission',
            id: 'perm-1',
            tool_call_id: null,
            title: 'Use Bash',
            tool: 'Bash',
            role: command('pnpm test'),
            detail: 'pnpm test',
            auto_approved: false,
          },
        },
      ],
      approvals: [
        {
          id: 'perm-1',
          threadId: 't1',
          status: 'pending',
          tool: 'execute',
          payload: JSON.stringify({
            title: 'Run command',
            what: 'pnpm test',
            why: 'Bash',
            okLabel: 'Approve',
            noLabel: 'Deny',
          }),
        },
      ],
      findingsReady: false,
      showLive: false,
    })
    expect(items[0]).toMatchObject({
      kind: 'approval',
      title: 'Run command',
      okLabel: 'Approve',
    })
  })

  it('shows a question only as its card, not as a hidden step', () => {
    const question = {
      question: 'Which colour?',
      header: 'Colour',
      options: [{ label: 'Red', description: 'Warm' }],
      multiSelect: false,
    }
    const items = mapEventsToTimeline({
      events: [
        {
          seq: 1,
          event: {
            type: 'toolCall',
            id: 'toolu_ask',
            title: 'Ask user question',
            role: { kind: 'other', name: 'Ask user question' },
            status: 'pending',
            locations: [],
            detail: null,
            hidden: true,
            parent_id: null,
          },
        },
        {
          seq: 2,
          event: {
            type: 'permission',
            id: 'q1',
            tool_call_id: 'toolu_ask',
            title: 'Use Ask user question',
            tool: 'Ask user question',
            role: { kind: 'other', name: 'Ask user question' },
            detail: null,
            auto_approved: false,
          },
        },
      ],
      approvals: [
        {
          id: 'q1',
          threadId: 't1',
          status: 'pending',
          tool: 'question',
          payload: JSON.stringify({
            title: 'Question',
            what: 'Which colour?',
            why: '',
            okLabel: 'Send',
            noLabel: 'Skip',
            questions: [question],
          }),
        },
      ],
      findingsReady: false,
      showLive: false,
    })
    expect(items).toHaveLength(1)
    expect(items[0]).toMatchObject({
      kind: 'approval',
      questions: [question],
      answers: {},
    })
  })

  it('shows a plan only as its approval, not as a hidden step', () => {
    const items = mapEventsToTimeline({
      events: [
        {
          seq: 1,
          event: {
            type: 'toolCall',
            id: 'toolu_enter',
            title: 'Enter plan mode',
            role: { kind: 'planMode' },
            status: 'completed',
            locations: [],
            detail: null,
          },
        },
        {
          seq: 2,
          event: {
            type: 'toolCall',
            id: 'toolu_plan',
            title: 'Exit plan mode',
            role: { kind: 'other', name: 'Exit plan mode' },
            status: 'inProgress',
            locations: [],
            detail: null,
            hidden: true,
          },
        },
        {
          seq: 3,
          event: {
            type: 'permission',
            id: 'plan-1',
            tool_call_id: 'toolu_plan',
            title: 'Use Exit plan mode',
            tool: 'Exit plan mode',
            role: { kind: 'other', name: 'Exit plan mode' },
            detail: null,
            auto_approved: false,
          },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
    })
    expect(items.map((item) => item.kind)).toEqual(['toolRun', 'approval'])
    expect(items[0]).toMatchObject({
      steps: [{ title: 'Switched to plan mode' }],
    })
  })

  it('inserts items before live row', () => {
    const items = mapEventsToTimeline({
      events: [
        {
          seq: 1,
          event: { type: 'messageChunk', role: 'user', text: 'Go' },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: true,
    })
    expect(items.at(-1)?.kind).toBe('live')
    expect(items[0]?.kind).toBe('user')
  })
})

describe('tool steps by role', () => {
  function stepFor(role: ToolRole, detail: string | null, title = 'Tool') {
    const items = mapEventsToTimeline({
      events: [
        {
          seq: 1,
          event: {
            type: 'toolCall',
            id: 'toolu_1',
            title,
            role,
            status: 'inProgress',
            locations: [],
            detail,
          },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
    })
    const run = items[0]
    if (run?.kind !== 'toolRun') throw new Error('expected a tool run')
    return run.steps[0]
  }

  it('shows an MCP call by its server, with its action before the detail', () => {
    expect(
      stepFor({ kind: 'mcp', server: 'Trello', action: 'Read card' }, null),
    ).toMatchObject({ title: 'Used Trello', detail: 'Read card', quiet: true })
    expect(
      stepFor({ kind: 'mcp', server: 'Github', action: 'Get pr' }, 'cormux#3'),
    ).toMatchObject({ title: 'Used Github', detail: 'Get pr · cormux#3' })
  })

  it('shows other tools by the title the engine gave them', () => {
    expect(
      stepFor({ kind: 'other', name: 'Todo write' }, null, 'Todo write'),
    ).toMatchObject({ title: 'Todo write', icon: 'tool' })
  })

  it('picks a command icon from the role, not from words in the command', () => {
    const ran = (text: string) => stepFor(command(text), null, text)
    expect(ran('npm run preview')).toMatchObject({ icon: 'terminal' })
    expect(ran('git pull --rebase')).toMatchObject({ icon: 'terminal' })
    expect(ran('pnpm test')).toMatchObject({ icon: 'check' })
  })

  it('shows a web fetch as a quiet line with a short URL', () => {
    expect(
      stepFor(
        { kind: 'fetch', url: 'https://www.example.com/docs/page/' },
        'https://www.example.com/docs/page/',
      ),
    ).toMatchObject({
      icon: 'globe',
      title: 'Fetched',
      detail: 'example.com/docs/page',
      rawDetail: 'https://www.example.com/docs/page/',
      quiet: true,
    })
  })

  it('shows a skill by name as a quiet line', () => {
    expect(
      stepFor({ kind: 'skill', name: 'code-review' }, 'code-review'),
    ).toMatchObject({
      icon: 'skill',
      title: 'Used skill',
      detail: 'code-review',
      quiet: true,
    })
  })

  it('shows a tool lookup as a quiet line', () => {
    expect(stepFor({ kind: 'toolLookup' }, 'select:Read')).toMatchObject({
      icon: 'search',
      title: 'Looked up tools',
      detail: 'select:Read',
      quiet: true,
    })
  })

  it('leaves a hidden call out', () => {
    const items = mapEventsToTimeline({
      events: [
        {
          seq: 1,
          event: {
            type: 'toolCall',
            id: 'toolu_1',
            title: 'Ask user question',
            role: { kind: 'other', name: 'Ask user question' },
            status: 'pending',
            locations: [],
            detail: null,
            hidden: true,
          },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
    })
    expect(items).toEqual([])
  })
})

describe('item ids', () => {
  const read = (id: string): TranscriptEvent => ({
    type: 'toolCall',
    id,
    title: 'Read',
    role: { kind: 'read' },
    status: 'completed',
    locations: [],
    detail: null,
  })
  const events: TranscriptEvent[] = [
    { type: 'messageChunk', role: 'user', text: 'one' },
    { type: 'messageChunk', role: 'agent', text: 'first' },
    { type: 'turnEnd', stop_reason: 'end_turn', error: null },
    { type: 'messageChunk', role: 'user', text: 'two' },
    read('toolu_9'),
    { type: 'messageChunk', role: 'agent', text: 'second' },
  ]
  const rows = events.map((event, index) => ({
    seq: index + 1,
    atMs: 0,
    event,
  }))
  const input = { approvals: [], findingsReady: false, showLive: false }

  it('come from where the store saved each item and from tool call ids', () => {
    const ids = mapEventsToTimeline({ ...input, events: rows }).map(
      (item) => item.id,
    )
    expect(ids).toEqual([
      'user-1',
      'thought-2',
      'user-4',
      'run-toolu_9',
      'thought-6',
    ])
  })

  it('stay put when older events load in front', () => {
    const builder = new TimelineBuilder()
    const newest = builder.build({ ...input, events: rows.slice(3) })
    const all = builder.build({ ...input, events: rows })
    expect(all.slice(-newest.length).map((item) => item.id)).toEqual(
      newest.map((item) => item.id),
    )
  })
})

describe('shouldShowLiveRow', () => {
  it('shows live while running or provisioning', () => {
    expect(shouldShowLiveRow('running', false)).toBe(true)
    expect(shouldShowLiveRow('provisioning', false)).toBe(true)
    expect(shouldShowLiveRow('idle', false)).toBe(false)
  })

  it('keeps step ids unique for repeated branch switches and a step written twice', () => {
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
    const events = [
      switched('step-1', 'feat/test'),
      switched('step-2', 'feat/colors'),
      switched('step-3', 'feat/test'),
      opened,
      opened,
    ].map((event, index) => ({ seq: index + 1, event }))
    const items = mapEventsToTimeline({
      events,
      approvals: [],
      findingsReady: false,
      showLive: false,
    })
    const ids = items.flatMap((item) =>
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
