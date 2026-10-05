import { describe, expect, it } from 'vitest'
import {
  mapEventsToTimeline,
  shouldShowLiveRow,
} from './map-events-to-timeline'
import { agentEventsForThread } from './parse-agent-event'

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
            name: 'Bash',
            kind: 'execute',
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
            kind: null,
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
            name: 'Read',
            kind: 'read',
            status: 'inProgress',
            locations: ['/repo/README.md'],
            detail: '/repo/README.md',
          },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
      liveToolTitle: null,
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
      liveToolTitle: null,
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
            name: null,
            kind: 'read',
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
            name: null,
            kind: 'execute',
            status: 'completed',
            locations: [],
            detail: null,
          },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
      liveToolTitle: null,
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
    const command =
      '/Users/me/.cursor/skills/impeccable comp-spec --comp mocks/01-pad-on-desk.png --grid'
    const items = mapEventsToTimeline({
      events: [
        {
          seq: 1,
          event: {
            type: 'toolCall',
            id: 'cursor-shell',
            title: `\`${command}\``,
            name: null,
            kind: 'execute',
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
            name: 'Bash',
            kind: 'execute',
            status: 'completed',
            locations: [],
            detail: 'allowed by settings',
          },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
      liveToolTitle: null,
    })
    const steps = items[0]?.kind === 'toolRun' ? items[0].steps : []
    expect(steps[0]).toMatchObject({
      title: 'Ran',
      detail: `${command.slice(0, 50).trimEnd()}…`,
      rawDetail: command,
      quiet: true,
    })
    expect(steps[1]).toMatchObject({ title: 'Ran a command', quiet: true })
  })

  it('fills a Cursor edit from its later update', () => {
    const editCall = (id: string) => ({
      type: 'toolCall' as const,
      id,
      title: 'Edit File',
      name: null,
      kind: 'edit' as const,
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
            kind: null,
            status: 'completed',
            locations: ['src/lib/auth.ts'],
          },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
      liveToolTitle: null,
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
      liveToolTitle: null,
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
      liveToolTitle: null,
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
            name: null,
            kind: 'read',
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
            name: null,
            kind: 'search',
            status: 'completed',
            locations: [],
            detail: 'pattern',
          },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
      liveToolTitle: null,
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
            name: null,
            kind: 'edit',
            status: 'completed',
            locations: ['src/lib/auth.ts'],
            detail: null,
          },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
      liveToolTitle: null,
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
            tool_name: 'Bash',
            kind: 'execute',
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
      liveToolTitle: null,
    })
    expect(items[0]).toMatchObject({
      kind: 'approval',
      title: 'Run command',
      okLabel: 'Approve',
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
      liveToolTitle: null,
    })
    expect(items.at(-1)?.kind).toBe('live')
    expect(items[0]?.kind).toBe('user')
  })
})

describe('named tool steps', () => {
  function stepFor(name: string, detail: string | null) {
    const items = mapEventsToTimeline({
      events: [
        {
          seq: 1,
          event: {
            type: 'toolCall',
            id: 'toolu_1',
            title: name,
            name,
            kind: 'other',
            status: 'inProgress',
            locations: [],
            detail,
          },
        },
      ],
      approvals: [],
      findingsReady: false,
      showLive: false,
      liveToolTitle: null,
    })
    const run = items[0]
    if (run?.kind !== 'toolRun') throw new Error('expected a tool run')
    return run.steps[0]
  }

  it('shows an MCP call by its server and tool', () => {
    expect(
      stepFor('mcp__claude_ai_Trello__trelloReadCard', null),
    ).toMatchObject({
      title: 'Used Trello',
      detail: 'Read card',
      quiet: true,
    })
  })

  it('turns snake and kebab case names into words', () => {
    expect(stepFor('mcp__linear-server__list_issues', null)).toMatchObject({
      title: 'Used Linear server',
      detail: 'List issues',
    })
    expect(stepFor('mcp__github__github_get_pr', 'cormux#3')).toMatchObject({
      title: 'Used Github',
      detail: 'Get pr · cormux#3',
    })
  })

  it('shows other tools by name in words', () => {
    expect(stepFor('TodoWrite', null)).toMatchObject({ title: 'Todo write' })
  })

  it('shows a web fetch as a quiet line with a short URL', () => {
    expect(
      stepFor('WebFetch', 'https://www.example.com/docs/page/'),
    ).toMatchObject({
      icon: 'globe',
      title: 'Fetched',
      detail: 'example.com/docs/page',
      rawDetail: 'https://www.example.com/docs/page/',
      quiet: true,
    })
  })

  it('shows a skill by name as a quiet line', () => {
    expect(stepFor('Skill', 'code-review')).toMatchObject({
      icon: 'skill',
      title: 'Used skill',
      detail: 'code-review',
      quiet: true,
    })
  })

  it('shows ToolSearch as a quiet lookup', () => {
    expect(stepFor('ToolSearch', 'select:Read')).toMatchObject({
      icon: 'search',
      title: 'Looked up tools',
      detail: 'select:Read',
      quiet: true,
    })
  })
})

describe('shouldShowLiveRow', () => {
  it('shows live while running or provisioning', () => {
    expect(shouldShowLiveRow('running', false)).toBe(true)
    expect(shouldShowLiveRow('provisioning', false)).toBe(true)
    expect(shouldShowLiveRow('idle', false)).toBe(false)
  })

  it('keeps step ids unique for repeated branch switches and replayed tool calls', () => {
    const legacy = (seq: number, title: string) => ({
      id: seq,
      threadId: 't',
      seq,
      kind: 'tool',
      payload: JSON.stringify({ icon: 'branch', title }),
      createdAt: '2026-09-29 04:26:22',
    })
    const events = [
      ...agentEventsForThread(
        [
          legacy(1, 'Switched to `feat/test`'),
          legacy(2, 'Switched to `feat/colors`'),
          legacy(3, 'Switched to `feat/test`'),
        ],
        't',
      ),
      ...[4, 5].map((seq) => ({
        seq,
        event: {
          type: 'toolCall' as const,
          id: 'call-1',
          title: 'Read file',
          name: null,
          kind: 'read' as const,
          status: 'completed' as const,
          locations: [],
          detail: null,
        },
      })),
    ]
    const items = mapEventsToTimeline({
      events,
      approvals: [],
      findingsReady: false,
      showLive: false,
      liveToolTitle: null,
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
