import { describe, expect, it } from 'vitest'
import type { TimelineStep, ToolRole, TranscriptEvent } from '$lib/ipc/bindings'
import { TimelineBuilder } from '$lib/thread/timeline/builder'
import type { RunStep, TimelineItem } from '$lib/thread/timeline-types'
import { stepIcon } from './step-icons'
import {
  agentState,
  iconTone,
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

const findingsFixed: TimelineStep = {
  kind: 'findingsFixed',
  count: 2,
  files: ['a.rs'],
}

describe('core step icons', () => {
  it('draws a step saved by an older build with the icon it carries', () => {
    expect(
      stepIcon({
        kind: 'legacy',
        title: 'Read 3 files',
        icon: 'file',
        run: false,
      }),
    ).toBe('file')
  })

  it('draws a kind this build does not know plainly', () => {
    expect(stepIcon({ kind: 'somethingNew' } as unknown as TimelineStep)).toBe(
      'tool',
    )
  })

  it('draws a step among tool steps with its icon, standing out', () => {
    const steps = stepsOf(call('r', { kind: 'read' }), step(findingsFixed))
    expect(toolIcon(steps[1]!)).toBe('check')
    expect(segmentSteps(steps).map((segment) => segment.quiet)).toEqual([
      true,
      false,
    ])
    expect(iconTone(steps[1]!)).toBe('text-success')
  })
})

describe('tool step looks', () => {
  const iconOf = (role: ToolRole) => toolIcon(stepsOf(call('t', role))[0]!)

  it('picks a command icon from the role, not from words in the command', () => {
    expect(iconOf(command('npm run preview'))).toBe('terminal')
    expect(iconOf(command('git pull --rebase'))).toBe('terminal')
    expect(iconOf(command('pnpm test'))).toBe('check')
  })

  it('tells a deleted file from an edited one', () => {
    expect(iconOf({ kind: 'edit', verb: 'Deleted' })).toBe('trash')
    expect(iconOf({ kind: 'edit', verb: 'Edited' })).toBe('pencil')
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

  it('claims nothing about fixed findings that the core never checked', () => {
    const [fixed] = stepsOf(step(findingsFixed))
    expect(stepChips(fixed!, false)).toEqual([])
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
