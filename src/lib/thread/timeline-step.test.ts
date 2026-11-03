import { describe, expect, it } from 'vitest'
import type { AgentEvent, TimelineStep } from '$lib/ipc/bindings'
import { splitConversations } from './conversations'
import { mapEventsToTimeline, TimelineBuilder } from './map-events-to-timeline'
import { hasSessionToClear } from './new-session'
import { parseAgentEventPayload } from './parse-agent-event'
import type { TimelineItem } from './timeline-types'

function render(...events: AgentEvent[]): TimelineItem[] {
  return mapEventsToTimeline({
    events: events.map((event, index) => ({
      seq: index + 1,
      atMs: 1000,
      event,
    })),
    approvals: [],
    findingsReady: false,
    showLive: false,
  })
}

function typed(
  step: TimelineStep,
  title: string,
  detail: string | null = null,
  id = `step-${step.kind}`,
): AgentEvent {
  return { type: 'step', id, title, detail, step }
}

const toolCall = (id: string, title: string, detail: string | null = null) =>
  ({
    type: 'toolCall',
    id,
    title,
    name: null,
    kind: 'other',
    status: 'completed',
    locations: [],
    detail,
  }) satisfies AgentEvent

const CHIPS = [
  { label: 'Tests pass', tone: 'success' },
  { label: 'Committed locally, not pushed', tone: 'muted' },
]

type Sample = {
  step: TimelineStep
  title: string
  icon: string
  /** How the row was saved before steps had a kind. */
  legacy: AgentEvent
}

const dividerRow = (id: string, title: string) => toolCall(id, title)

/** Keyed by kind, so a new kind doesn't compile until it has a sample here. */
const SAMPLES: { [K in TimelineStep['kind']]: Sample } = {
  agentStopped: {
    step: { kind: 'agentStopped' },
    title: 'You stopped the agent',
    icon: 'stop',
    legacy: dividerRow('control-1', 'You stopped the agent'),
  },
  agentPaused: {
    step: { kind: 'agentPaused' },
    title: 'You paused the agent',
    icon: 'pause',
    legacy: dividerRow('control-1', 'You paused the agent'),
  },
  agentResumed: {
    step: { kind: 'agentResumed' },
    title: 'You resumed the agent',
    icon: 'play',
    legacy: dividerRow('control-1', 'You resumed the agent'),
  },
  sessionStarted: {
    step: { kind: 'sessionStarted', ended_session: 's1' },
    title: 'Started a new session',
    icon: 'session',
    legacy: dividerRow('control-1', 'Started a new session'),
  },
  sessionResumed: {
    step: { kind: 'sessionResumed', conversation: 'c', ended_session: null },
    title: 'Resumed an earlier conversation',
    icon: 'tool',
    legacy: dividerRow('control-1', 'Resumed an earlier conversation'),
  },
  startedByThread: {
    step: { kind: 'startedByThread', workspace: 'Auth' },
    title: 'Started by Auth',
    icon: 'tool',
    legacy: dividerRow('control-1', 'Started by Auth'),
  },
  messageFromThread: {
    step: { kind: 'messageFromThread', workspace: 'Auth' },
    title: 'Message from Auth',
    icon: 'tool',
    legacy: dividerRow('control-1', 'Message from Auth'),
  },
  reportFromThread: {
    step: { kind: 'reportFromThread', title: 'Auth' },
    title: 'Report from Auth',
    icon: 'tool',
    legacy: dividerRow('control-1', 'Report from Auth'),
  },
  fileDeleted: {
    step: { kind: 'fileDeleted', path: 'a.rs' },
    title: 'Deleted a.rs',
    icon: 'trash',
    legacy: dividerRow('control-1', 'Deleted a.rs'),
  },
  migrationApplied: {
    step: { kind: 'migrationApplied' },
    title: 'Applied migration',
    icon: 'database',
    legacy: dividerRow('control-1', 'Applied migration'),
  },
  suggestionsForwarded: {
    step: { kind: 'suggestionsForwarded' },
    title: 'Sent suggestions to Lead',
    icon: 'pr',
    legacy: dividerRow('control-1', 'Sent suggestions to Lead'),
  },
  suggestionsSent: {
    step: { kind: 'suggestionsSent', approval_id: 'a', what: 'a.rs' },
    title: 'Suggestions sent',
    icon: 'pr',
    legacy: dividerRow('review-1', 'Suggestions sent'),
  },
  branchSwitched: {
    step: {
      kind: 'branchSwitched',
      branch: 'b',
      change: { kind: 'stackSynced' },
    },
    title: 'Switched to `b`',
    icon: 'branch',
    legacy: dividerRow('legacy-1', 'Switched to `b`'),
  },
  commitsPulled: {
    step: { kind: 'commitsPulled', count: 2, base: 'main', branch: 'b' },
    title: 'Pulled 2 commits from main',
    icon: 'download',
    legacy: dividerRow('legacy-1', 'Pulled 2 commits from main'),
  },
  branchRebased: {
    step: { kind: 'branchRebased', branch: 'b', base: 'm', replayed_onto: 1 },
    title: 'Rebased b onto m',
    icon: 'branch',
    legacy: dividerRow('legacy-1', 'Rebased b onto m'),
  },
  worktreeCreated: {
    step: { kind: 'worktreeCreated', branch: 'b', base: 'm' },
    title: 'Created worktree',
    icon: 'branch',
    legacy: dividerRow('legacy-1', 'Created worktree'),
  },
  worktreeReady: {
    step: { kind: 'worktreeReady', repo: 'app', setup_commands: 2 },
    title: 'Ran worktree setup',
    icon: 'branch',
    legacy: dividerRow('legacy-1', 'Ran worktree setup'),
  },
  prOpened: {
    step: { kind: 'prOpened', number: 12, branch: 'b', base: 'm' },
    title: 'Opened PR #12',
    icon: 'pr',
    legacy: dividerRow('pr-open-12', 'Opened PR #12'),
  },
  prCheckedOut: {
    step: {
      kind: 'prCheckedOut',
      number: 7,
      head: 'b',
      base: 'm',
      files_changed: 3,
    },
    title: 'Checked out #7',
    icon: 'pr',
    legacy: dividerRow('legacy-1', 'Checked out #7'),
  },
  reviewSubmitted: {
    step: { kind: 'reviewSubmitted', number: 7 },
    title: 'Submitted review on #7',
    icon: 'pr',
    legacy: dividerRow('legacy-1', 'Submitted review on #7'),
  },
  reviewFindingsUnreadable: {
    step: { kind: 'reviewFindingsUnreadable', reason: 'no block' },
    title: 'Couldn’t read the review findings',
    icon: 'tool',
    legacy: dividerRow('legacy-1', 'Couldn’t read the review findings'),
  },
  findingsReceived: {
    step: { kind: 'findingsReceived', count: 2, titles: ['A', 'B'] },
    title: 'Received 2 findings from the review',
    icon: 'list',
    legacy: toolCall('finding-1', 'Received 2 findings from the review'),
  },
  findingsFixed: {
    step: { kind: 'findingsFixed', count: 2, files: ['a.rs'] },
    title: 'Fixed 2 findings',
    icon: 'check',
    legacy: toolCall('finding-1', 'Fixed 2 findings'),
  },
}

function firstRow(items: TimelineItem[]) {
  const item = items[0]
  if (item?.kind === 'event') return item
  if (item?.kind === 'toolRun') return item.steps[0]
  throw new Error(`nothing to compare: ${item?.kind}`)
}

/** What a row looks like, without the id that differs between its two forms. */
function look(event: AgentEvent) {
  return { ...firstRow(render(event)), id: undefined }
}

describe('typed steps', () => {
  for (const [kind, sample] of Object.entries(SAMPLES)) {
    it(`gives ${kind} its icon`, () => {
      expect(firstRow(render(typed(sample.step, sample.title)))).toMatchObject({
        icon: sample.icon,
        title: sample.title,
      })
    })

    it(`draws ${kind} the same as a row saved without a kind`, () => {
      expect(look(typed(sample.step, sample.title))).toEqual(
        look(sample.legacy),
      )
    })

    it(`keeps ${kind}'s look when its title is reworded`, () => {
      const reworded = 'Something else entirely'
      const original = look(typed(sample.step, sample.title))
      expect(look(typed(sample.step, reworded))).toEqual({
        ...original,
        title: reworded,
      })
    })
  }

  it('gives fixed findings their chips from the kind, not the words', () => {
    const items = render(
      typed(SAMPLES.findingsFixed.step, 'All sorted', 'a.rs'),
    )
    expect(items).toMatchObject([
      {
        kind: 'toolRun',
        steps: [
          {
            kind: 'tool',
            icon: 'check',
            title: 'All sorted',
            detail: 'a.rs',
            tone: 'success',
            chips: CHIPS,
          },
        ],
      },
    ])
  })

  it('does not read meaning from a title that looks like another step', () => {
    const paused = typed(
      { kind: 'startedByThread', workspace: 'you paused #12' },
      'Started by you paused #12',
    )
    expect(firstRow(render(paused))).toMatchObject({ icon: 'tool' })
  })

  it('joins the tool steps around it when it runs with them', () => {
    const items = render(
      toolCall('toolu_1', 'Thinking it over'),
      typed(SAMPLES.findingsReceived.step, 'Got them', 'A · B'),
    )
    expect(items).toHaveLength(1)
    expect(items[0]).toMatchObject({ kind: 'toolRun' })
  })

  it('keeps the first of a step written twice, as tool calls did', () => {
    const step = SAMPLES.prOpened.step
    const items = render(
      typed(step, 'Opened PR #12', null, 'pr-open-12'),
      typed(step, 'Opened PR #12', null, 'pr-open-12'),
    )
    expect(items).toHaveLength(1)
  })

  it('treats a step repeating an id from a saved tool call as that call', () => {
    const items = render(
      toolCall('pr-open-12', 'Opened PR #12'),
      typed(SAMPLES.prOpened.step, 'Opened PR #12', null, 'pr-open-12'),
    )
    expect(items).toHaveLength(1)
  })

  it('reads back from the store as a step', () => {
    const event = typed(SAMPLES.agentPaused.step, 'You paused the agent')
    expect(parseAgentEventPayload(JSON.stringify(event), 4)).toEqual(event)
  })

  it('still shows a kind this build does not know', () => {
    const event = {
      type: 'step',
      id: 'step-new',
      title: 'From the future',
      detail: null,
      step: { kind: 'somethingNew' },
    } as unknown as AgentEvent
    expect(render(event)).toMatchObject([
      { kind: 'event', icon: 'tool', title: 'From the future' },
    ])
  })

  it('rebuilds the same way incrementally', () => {
    const turnEnd: AgentEvent = {
      type: 'turnEnd',
      stop_reason: 'end_turn',
      error: null,
    }
    const step = typed(SAMPLES.prOpened.step, 'PR is up', null, 'pr-open-12')
    const rows = [step, turnEnd, step].map((event, index) => ({
      seq: index + 1,
      atMs: 0,
      event,
    }))
    const input = { approvals: [], findingsReady: false, showLive: false }
    const builder = new TimelineBuilder()
    builder.build({ ...input, events: rows.slice(0, 2) })
    expect(builder.build({ ...input, events: rows })).toEqual(
      mapEventsToTimeline({ ...input, events: rows }),
    )
  })

  it('splits conversations on session steps', () => {
    const ask = (text: string): AgentEvent => ({
      type: 'messageChunk',
      role: 'user',
      text,
    })
    const started = typed(
      { kind: 'sessionStarted', ended_session: 'sess-1' },
      'Rephrased',
    )
    const resumed = typed(
      {
        kind: 'sessionResumed',
        conversation: 'first',
        ended_session: 'sess-2',
      },
      'Also rephrased',
    )
    const rows = [ask('One'), started, ask('Two'), resumed, ask('Three')].map(
      (event, index) => ({ seq: index + 1, atMs: index + 1, event }),
    )
    const { current, earlier } = splitConversations(rows)
    expect(current).toMatchObject({ id: 'first', prompts: 2 })
    expect(earlier).toMatchObject([
      { id: 'after-2', firstPrompt: 'Two', sessionId: 'sess-2' },
    ])
  })

  it('has no session to clear right after a new session step', () => {
    const started = typed(SAMPLES.sessionStarted.step, 'Rephrased')
    const paused = typed(SAMPLES.agentPaused.step, 'You paused the agent')
    expect(hasSessionToClear([{ event: started }])).toBe(false)
    expect(hasSessionToClear([{ event: paused }])).toBe(true)
  })
})

describe('rows saved without a kind', () => {
  const stored = (payload: object, seq = 1) =>
    parseAgentEventPayload(JSON.stringify(payload), seq)!

  it('reads the title of a bare row and ignores its icon', () => {
    const row = stored({
      icon: 'terminal',
      title: 'Ran worktree setup',
      detail: '2 commands',
      chip: 'Worktree ready',
    })
    expect(render(row)).toMatchObject([
      {
        kind: 'event',
        id: 'legacy-1',
        icon: 'branch',
        title: 'Ran worktree setup',
        detail: '2 commands',
      },
    ])
  })

  const bare: [string, string][] = [
    ['Checked out #7', 'pr'],
    ['Submitted review on #7', 'pr'],
    ['Couldn’t read the review findings', 'tool'],
    ['Switched to `feat/a`', 'branch'],
    ['Rebased a onto b', 'branch'],
    ['Pulled 2 commits from main', 'download'],
    ['Created worktree', 'branch'],
    ['Worktree ready', 'branch'],
    ['Read 3 changed files', 'file'],
    ['Opened index.ts', 'file'],
    ['Approved the plan', 'check'],
  ]
  for (const [title, icon] of bare) {
    it(`gives "${title}" the ${icon} icon`, () => {
      expect(render(stored({ title }))).toMatchObject([
        { kind: 'event', icon, title },
      ])
    })
  }

  it('shows an opened PR as a divider, where chips never appear', () => {
    expect(render(toolCall('pr-open-12', 'Opened PR #12'))).toEqual([
      {
        kind: 'event',
        id: 'pr-open-12',
        icon: 'pr',
        title: 'Opened PR #12',
        detail: undefined,
        seq: 1,
        atMs: 1000,
      },
    ])
  })

  it('gives a fixed-findings row its chips', () => {
    expect(
      render(toolCall('finding-1', 'Fixed 3 findings', 'a.rs')),
    ).toMatchObject([
      {
        kind: 'toolRun',
        steps: [{ icon: 'check', chips: CHIPS, tone: 'success' }],
      },
    ])
  })

  it('gives a received-findings row the list icon and no chips', () => {
    const items = render(
      toolCall('finding-1', 'Received 2 findings from the review'),
    )
    expect(firstRow(items)).toMatchObject({ icon: 'list', tone: 'success' })
    expect(firstRow(items)).not.toHaveProperty('chips.0')
  })

  it('gives any other opened-PR tool call its checks chip', () => {
    expect(
      firstRow(render(toolCall('toolu_1', 'Opened PR #12'))),
    ).toMatchObject({ chips: [{ label: 'Checks running' }], tone: 'success' })
  })

  it('still marks a saved new session and what it ended', () => {
    const detail = JSON.stringify({ endedSession: 'sess-1' })
    const marker = toolCall('control-1', 'Started a new session', detail)
    const ask: AgentEvent = { type: 'messageChunk', role: 'user', text: 'Hi' }
    const rows = [ask, marker].map((event, index) => ({
      seq: index + 1,
      atMs: index + 1,
      event,
    }))
    expect(splitConversations(rows).earlier).toMatchObject([
      { sessionId: 'sess-1' },
    ])
    expect(hasSessionToClear([{ event: marker }])).toBe(false)
  })
})
