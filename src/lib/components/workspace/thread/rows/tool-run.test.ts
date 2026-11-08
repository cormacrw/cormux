import { describe, expect, it } from 'vitest'
import type { TimelineStep, ToolRole, TranscriptEvent } from '$lib/ipc/bindings'
import { TimelineBuilder } from '$lib/thread/timeline/builder'
import type { RunStep, TimelineItem } from '$lib/thread/timeline-types'
import type { IconName } from './icon-name'
import { stepIcon } from './step-icons'
import {
  agentState,
  iconTone,
  isQuiet,
  newChips,
  runCollapses,
  segmentSteps,
  stepChips,
  toolIcon,
} from './tool-run'

function build(...events: TranscriptEvent[]): TimelineItem[] {
  return new TimelineBuilder().build({
    events: events.map((event, index) => ({ seq: index + 1, event })),
    approvals: [],
    findingsFor: null,
  }).items
}

function stepsOf(...events: TranscriptEvent[]): RunStep[] {
  const run = build(...events)[0]
  if (run?.kind !== 'toolRun') throw new Error(`not a run: ${run?.kind}`)
  return run.steps
}

const call = (
  id: string,
  role: ToolRole,
  status: 'inProgress' | 'completed' | 'failed' = 'completed',
  parent?: string,
): TranscriptEvent => ({
  type: 'toolCall',
  id,
  title: 'Tool',
  role,
  status,
  locations: [],
  detail: null,
  parent_id: parent ?? null,
})

const command = (text: string): ToolRole => ({
  kind: 'command',
  command: text,
  description: null,
})

const autoApproved: TranscriptEvent = {
  type: 'permission',
  id: 'perm',
  tool_call_id: null,
  title: 'Use Read',
  tool: 'Read',
  role: { kind: 'read' },
  detail: null,
  auto_approved: true,
}

const step = (kind: TimelineStep, id = 'step'): TranscriptEvent => ({
  type: 'step',
  id,
  title: 'Something the core did',
  detail: null,
  step: kind,
})

/** Keyed by kind, so a new kind doesn't compile until it has an icon here. */
const STEP_ICONS: {
  [K in TimelineStep['kind']]: [Extract<TimelineStep, { kind: K }>, IconName]
} = {
  agentStopped: [{ kind: 'agentStopped' }, 'stop'],
  agentPaused: [{ kind: 'agentPaused' }, 'pause'],
  agentResumed: [{ kind: 'agentResumed' }, 'play'],
  sessionStarted: [{ kind: 'sessionStarted', ended_session: 's1' }, 'session'],
  sessionResumed: [
    { kind: 'sessionResumed', conversation: 'c', ended_session: null },
    'tool',
  ],
  startedByThread: [{ kind: 'startedByThread', workspace: 'Auth' }, 'tool'],
  messageFromThread: [{ kind: 'messageFromThread', workspace: 'Auth' }, 'tool'],
  reportFromThread: [{ kind: 'reportFromThread', title: 'Auth' }, 'tool'],
  fileDeleted: [{ kind: 'fileDeleted', path: 'a.rs' }, 'trash'],
  migrationApplied: [{ kind: 'migrationApplied' }, 'database'],
  suggestionsForwarded: [{ kind: 'suggestionsForwarded' }, 'pr'],
  suggestionsSent: [
    { kind: 'suggestionsSent', approval_id: 'a', what: 'a.rs' },
    'pr',
  ],
  branchSwitched: [
    { kind: 'branchSwitched', branch: 'b', change: { kind: 'stackSynced' } },
    'branch',
  ],
  commitsPulled: [
    { kind: 'commitsPulled', count: 2, base: 'main', branch: 'b' },
    'download',
  ],
  branchRebased: [
    { kind: 'branchRebased', branch: 'b', base: 'm', replayed_onto: 1 },
    'branch',
  ],
  worktreeCreated: [
    { kind: 'worktreeCreated', branch: 'b', base: 'm' },
    'branch',
  ],
  worktreeReady: [
    { kind: 'worktreeReady', repo: 'app', setup_commands: 2 },
    'branch',
  ],
  prOpened: [{ kind: 'prOpened', number: 12, branch: 'b', base: 'm' }, 'pr'],
  prCheckedOut: [
    { kind: 'prCheckedOut', number: 7, head: 'b', base: 'm', files_changed: 3 },
    'pr',
  ],
  reviewSubmitted: [{ kind: 'reviewSubmitted', number: 7 }, 'pr'],
  reviewFindingsUnreadable: [
    { kind: 'reviewFindingsUnreadable', reason: 'no block' },
    'tool',
  ],
  findingsReceived: [
    { kind: 'findingsReceived', count: 2, titles: ['A', 'B'] },
    'list',
  ],
  findingsFixed: [
    { kind: 'findingsFixed', count: 2, files: ['a.rs'] },
    'check',
  ],
  legacy: [
    { kind: 'legacy', title: 'Read 3 changed files', icon: 'file', run: false },
    'file',
  ],
}

describe('core step icons', () => {
  for (const [kind, [sample, icon]] of Object.entries(STEP_ICONS)) {
    it(`draws ${kind} with its icon`, () => {
      expect(stepIcon(sample)).toBe(icon)
    })
  }

  it('draws a kind this build does not know plainly', () => {
    expect(stepIcon({ kind: 'somethingNew' } as unknown as TimelineStep)).toBe(
      'tool',
    )
  })

  it('draws a step among tool steps with the same icon, standing out', () => {
    const [fixed] = stepsOf(step(STEP_ICONS.findingsFixed[0]))
    expect(toolIcon(fixed!)).toBe('check')
    expect(isQuiet(fixed!)).toBe(false)
    expect(iconTone(fixed!)).toBe('text-success')
  })
})

describe('tool step looks', () => {
  const lookOf = (role: ToolRole) => {
    const [only] = stepsOf(call('t', role))
    return { icon: toolIcon(only!), quiet: isQuiet(only!) }
  }

  it('picks a command icon from the role, not from words in the command', () => {
    expect(lookOf(command('npm run preview')).icon).toBe('terminal')
    expect(lookOf(command('git pull --rebase')).icon).toBe('terminal')
    expect(lookOf(command('pnpm test')).icon).toBe('check')
  })

  it('draws each role with its icon, quietly unless it stands out', () => {
    expect(lookOf({ kind: 'read' })).toEqual({ icon: 'file', quiet: true })
    expect(lookOf({ kind: 'search' })).toEqual({ icon: 'search', quiet: true })
    expect(lookOf({ kind: 'toolLookup' })).toEqual({
      icon: 'search',
      quiet: true,
    })
    expect(lookOf({ kind: 'fetch', url: 'https://a.b' })).toEqual({
      icon: 'globe',
      quiet: true,
    })
    expect(lookOf({ kind: 'webSearch', query: 'q' })).toEqual({
      icon: 'globe',
      quiet: true,
    })
    expect(lookOf({ kind: 'skill', name: 'review' })).toEqual({
      icon: 'skill',
      quiet: true,
    })
    expect(lookOf({ kind: 'mcp', server: 'Trello', action: 'Read' })).toEqual({
      icon: 'tool',
      quiet: true,
    })
    expect(lookOf({ kind: 'other', name: 'Todo write' })).toEqual({
      icon: 'tool',
      quiet: false,
    })
    expect(lookOf({ kind: 'subagent', description: 'Explore' })).toEqual({
      icon: 'agent',
      quiet: false,
    })
    expect(lookOf({ kind: 'edit', verb: 'Deleted' })).toEqual({
      icon: 'trash',
      quiet: true,
    })
  })

  it('splits a run into quiet lines and steps that stand out', () => {
    const steps = stepsOf(
      call('a', { kind: 'read' }),
      call('b', { kind: 'search' }),
      call('c', { kind: 'other', name: 'Todo' }),
      call('d', { kind: 'read' }),
    )
    expect(
      segmentSteps(steps).map((segment) => [
        segment.quiet,
        segment.steps.map((one) => one.id),
      ]),
    ).toEqual([
      [true, ['a', 'b']],
      [false, ['c']],
      [true, ['d']],
    ])
  })
})

describe('chips', () => {
  it('marks a step that ran without asking', () => {
    const [read] = stepsOf(autoApproved, call('r', { kind: 'read' }))
    expect(stepChips(read!, false)).toEqual([
      { label: 'Read-only, auto-approved', tone: 'muted' },
    ])
  })

  it('marks scratch reads read-only with no worktree, but not its commands', () => {
    const [read, ran] = stepsOf(
      call('r', { kind: 'read' }),
      call('c', command('git log')),
    )
    expect(stepChips(read!, true)).toEqual([
      { label: 'Read-only, no worktree', tone: 'muted' },
    ])
    expect(stepChips(ran!, true)).toEqual([])
  })

  it('gives fixed findings their chips from the kind, not the words', () => {
    const [fixed] = stepsOf(step(STEP_ICONS.findingsFixed[0]))
    expect(stepChips(fixed!, false)).toEqual([
      { label: 'Tests pass', tone: 'success' },
      { label: 'Committed locally, not pushed', tone: 'muted' },
    ])
  })

  it('shows a run of reads its shared chip once', () => {
    const [first, second] = stepsOf(
      call('a', { kind: 'read' }),
      call('b', { kind: 'read' }),
    )
    expect(newChips(first!, undefined, true)).toHaveLength(1)
    expect(newChips(second!, first, true)).toEqual([])
  })
})

describe('agent steps', () => {
  const agent = { kind: 'subagent', description: 'Explore' } as const

  it('say how the agent is doing', () => {
    const [running] = stepsOf(
      call('a', agent, 'inProgress'),
      call('c1', { kind: 'read' }, 'completed', 'a'),
    )
    expect(agentState(running!)).toBe('running · 1 step')
    expect(iconTone(running!)).toBe('animate-pulse text-primary')
    const [failed] = stepsOf(call('a', agent, 'failed'))
    expect(agentState(failed!)).toBe('failed')
    expect(iconTone(failed!)).toBe('text-destructive')
  })

  it('keep a long run open while an agent in it works', () => {
    const reads = Array.from({ length: 6 }, (_, index) =>
      call(`r${index}`, { kind: 'read' }),
    )
    expect(runCollapses(stepsOf(...reads), false)).toBe(true)
    expect(runCollapses(stepsOf(...reads), true)).toBe(false)
    expect(
      runCollapses(stepsOf(...reads, call('a', agent, 'inProgress')), false),
    ).toBe(false)
  })
})
