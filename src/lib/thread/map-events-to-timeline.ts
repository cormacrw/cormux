import type { ApprovalRow, TranscriptEvent } from '$lib/ipc/bindings'
import { deepEqual } from '$lib/deep-equal'
import { memoByText } from './memo'
import { summarizeRun } from './reply/blocks/run'
import type {
  RunningAgent,
  TimelineChip,
  TimelineItem,
  ToolRunStep,
  ToolStepIcon,
} from './timeline-types'
import { agentIsRunning } from './timeline-types'
import { presentStep } from './timeline-step'

export type MapTimelineInput = {
  /**
   * In store order. `seq` is where the store saved the event, or, for one that is only streamed,
   * where the newest saved event before it is: items take their ids from it, so loading older
   * rows or a reload leaves every id where it was.
   */
  events: { seq: number; atMs?: number; event: TranscriptEvent }[]
  approvals: ApprovalRow[]
  findingsReady: boolean
  showLive: boolean
  /** A scratch runs on the main checkout in the engine's read-only mode. */
  scratch?: boolean
}

const SCRATCH_CHIP: TimelineChip = {
  label: 'Read-only, no worktree',
  tone: 'muted',
}

/** Under a minute: `Worked for 12s`. Otherwise `Worked for 2m 5s`. */
function workedForLabel(startMs: number, endMs: number): string {
  const elapsed = Math.max(0, endMs - startMs)
  const total = elapsed === 0 ? 0 : Math.max(1, Math.round(elapsed / 1000))
  const minutes = Math.floor(total / 60)
  const seconds = total % 60
  if (minutes === 0) return `Worked for ${seconds}s`
  return `Worked for ${minutes}m ${seconds}s`
}

type ToolCallEvent = Extract<TranscriptEvent, { type: 'toolCall' }>

function approvalState(
  row: ApprovalRow | undefined,
): 'pending' | 'approved' | 'denied' {
  if (!row) return 'pending'
  if (row.status === 'approved') return 'approved'
  if (row.status === 'denied' || row.status === 'dismissed') return 'denied'
  return 'pending'
}

// Looked up by id once per list, rather than scanned for every permission in the thread.
const approvalIndexes = new WeakMap<ApprovalRow[], Map<string, ApprovalRow>>()

function approvalById(approvals: ApprovalRow[], id: string) {
  let index = approvalIndexes.get(approvals)
  if (!index) {
    index = new Map(approvals.map((row) => [row.id, row]))
    approvalIndexes.set(approvals, index)
  }
  return index.get(id)
}

// A timeline rebuild re-reads every run, which doesn't change once written.
const summarizedRun = memoByText(summarizeRun)

function approvalFromPermission(
  event: Extract<TranscriptEvent, { type: 'permission' }>,
  seq: number,
  atMs: number,
  approvals: ApprovalRow[],
) {
  const row = approvalById(approvals, event.id)
  const payload = row?.payload ?? null
  const state = approvalState(row)
  return {
    kind: 'approval' as const,
    id: event.id,
    title: payload?.title ?? event.title,
    what: payload?.what ?? event.detail ?? event.tool,
    why: payload?.why ?? event.tool,
    okLabel: payload?.okLabel ?? 'Approve',
    noLabel: payload?.noLabel ?? 'Deny',
    state,
    doneAtMs: payload?.resolvedAtMs ?? null,
    questions: payload?.questions ?? [],
    answers: payload?.answers ?? {},
    seq,
    atMs,
  }
}

function toolStepFromCall(
  event: ToolCallEvent,
  seq: number,
  atMs: number,
  chips: TimelineChip[] | undefined,
): ToolRunStep {
  const { role } = event
  const detail = event.detail ?? undefined
  const quietStep = (
    icon: ToolStepIcon,
    title: string,
    shown: string | undefined,
  ): ToolRunStep => ({
    kind: 'tool',
    id: event.id,
    icon,
    title,
    detail: shown,
    chips,
    quiet: true,
    seq,
    atMs,
    rawDetail: detail,
  })
  switch (role.kind) {
    case 'edit':
      return {
        kind: 'edit',
        id: event.id,
        path: event.locations[0] ?? '',
        verb: role.verb,
        seq,
        atMs,
      }
    case 'subagent':
      return agentStep(event, seq, atMs, chips)
    case 'command':
      return commandStep(event, role.command, seq, atMs, chips)
    case 'fetch':
      return quietStep('globe', 'Fetched', shortUrl(event.detail))
    case 'webSearch':
      return quietStep('globe', 'Searched the web', detail)
    case 'skill':
      return quietStep('skill', 'Used skill', detail)
    case 'planMode':
      return quietStep('tool', 'Switched to plan mode', undefined)
    case 'toolLookup':
      return quietStep('search', 'Looked up tools', detail)
    case 'mcp':
      return quietStep(
        'tool',
        `Used ${role.server}`,
        detail ? `${role.action} · ${detail}` : role.action,
      )
    case 'read':
    case 'search':
    case 'other':
      return {
        kind: 'tool',
        id: event.id,
        icon:
          role.kind === 'read'
            ? 'file'
            : role.kind === 'search'
              ? 'search'
              : 'tool',
        title: event.title,
        detail: (event.detail ?? event.locations.join(', ')) || undefined,
        chips,
        tone: event.status === 'completed' ? 'success' : undefined,
        quiet: role.kind !== 'other',
        seq,
        atMs,
        rawDetail: detail,
      }
  }
}

// A subagent is detailed by the task's short description.
function agentStep(
  event: ToolCallEvent,
  seq: number,
  atMs: number,
  chips: TimelineChip[] | undefined,
): ToolRunStep {
  return {
    kind: 'tool',
    id: event.id,
    icon: 'agent',
    title: 'Agent',
    detail: event.detail ?? undefined,
    chips,
    tone: event.status === 'completed' ? 'success' : undefined,
    quiet: false,
    seq,
    atMs,
    status: event.status,
    agent: true,
  }
}

/** What a subagent did last, as its step reads in the timeline. */
function stepText(step: ToolRunStep): string {
  if (step.kind === 'edit') return `${step.verb} ${step.path.split('/').pop()}`
  return [step.title, step.detail].filter(Boolean).join(' ')
}

function runningAgents(agents: ToolRunStep[]): RunningAgent[] {
  return agents.filter(agentIsRunning).map((step) => {
    const children = step.kind === 'tool' ? (step.children ?? []) : []
    const last = children.at(-1)
    return {
      id: step.id,
      description:
        (step.kind === 'tool' ? step.detail : undefined) ?? 'Subagent',
      steps: children.length,
      latest: last ? stepText(last) : undefined,
    }
  })
}

/** `https://www.example.com/a/b` → `example.com/a/b`; the full URL stays in the tooltip. */
function shortUrl(url: string | null): string | undefined {
  if (!url) return undefined
  return url
    .replace(/^[a-z]+:\/\//i, '')
    .replace(/^www\./, '')
    .replace(/\/$/, '')
}

const COMMAND_PREVIEW_CHARS = 50

function withRunResult(
  step: ToolRunStep,
  event: ToolCallEvent,
  output: string | undefined,
): ToolRunStep {
  if (step.kind !== 'tool' || event.role.kind !== 'command' || !output)
    return step
  const run = summarizedRun(output)
  return run ? { ...step, run, output } : step
}

function commandStep(
  event: ToolCallEvent,
  command: string,
  seq: number,
  atMs: number,
  chips: TimelineChip[] | undefined,
): ToolRunStep {
  const preview =
    command.length > COMMAND_PREVIEW_CHARS
      ? `${command.slice(0, COMMAND_PREVIEW_CHARS).trimEnd()}…`
      : command
  return {
    kind: 'tool',
    id: event.id,
    // A test run gets a check, the way its summary reads.
    icon: command.toLowerCase().includes('test') ? 'check' : 'terminal',
    title: command ? 'Ran' : 'Ran a command',
    detail: preview || undefined,
    chips,
    quiet: true,
    seq,
    atMs,
    rawDetail: command || undefined,
  }
}

/** A run takes its id from its first step, whose id is the tool call's own. */
function flushRun(run: ToolRunStep[], items: TimelineItem[], seq: number) {
  const first = run[0]
  if (!first) return
  items.push({
    kind: 'toolRun',
    id: `run-${first.id}`,
    steps: [...run],
    seq,
  })
  run.length = 0
}

type TimelineEvent = MapTimelineInput['events'][number] & { output?: string }

// Engines fill in a tool call after announcing it, so fold each update into the call it belongs
// to. A step written twice (a PR opened again) keeps its id; the first one stands, so step ids,
// which key the rendered list, stay unique.
function mergeToolUpdates(events: TimelineEvent[]): TimelineEvent[] {
  const merged: TimelineEvent[] = []
  const callIndex = new Map<string, number>()
  for (const entry of events) {
    const { event } = entry
    if (event.type === 'toolCall' || event.type === 'step') {
      if (callIndex.has(event.id)) continue
      callIndex.set(event.id, merged.length)
      merged.push(entry)
      continue
    }
    if (event.type !== 'toolCallUpdate') {
      merged.push(entry)
      continue
    }
    const index = callIndex.get(event.id)
    const call = index === undefined ? undefined : merged[index]
    if (index === undefined || call?.event.type !== 'toolCall') continue
    merged[index] = {
      ...call,
      output: event.output ?? call.output,
      event: {
        ...call.event,
        title: event.title ?? call.event.title,
        status: event.status ?? call.event.status,
        locations: event.locations.length
          ? event.locations
          : call.event.locations,
      },
    }
  }
  return merged
}

type FoldState = {
  items: TimelineItem[]
  run: ToolRunStep[]
  // A subagent's steps can land runs after the step that spawned them.
  stepsById: Map<string, ToolRunStep>
  // Agent steps, for the live row. A background agent outlives the run that started it.
  agents: ToolRunStep[]
  runSeq: number
  pendingAutoChip: TimelineChip | undefined
  sealMessage: boolean
  // Set when the user sends the prompt; cleared when that turn ends.
  runStartedAt: number
}

function foldState(
  items: TimelineItem[] = [],
  pendingAutoChip?: TimelineChip,
  sealMessage = false,
  agents: ToolRunStep[] = [],
): FoldState {
  return {
    items,
    run: [],
    stepsById: new Map(),
    agents,
    runSeq: 0,
    pendingAutoChip,
    sealMessage,
    runStartedAt: 0,
  }
}

function foldEvent(
  state: FoldState,
  { seq, atMs = 0, event, output }: TimelineEvent,
  input: MapTimelineInput,
) {
  const { items, run, stepsById } = state
  switch (event.type) {
    case 'messageChunk': {
      flushRun(run, items, state.runSeq)
      if (event.role === 'user') {
        if (!state.runStartedAt && atMs) state.runStartedAt = atMs
        const last = items[items.length - 1]
        if (!state.sealMessage && last?.kind === 'user') {
          last.text += event.text
          last.seq = seq
        } else {
          items.push({
            kind: 'user',
            id: `user-${seq}`,
            text: event.text,
            seq,
            atMs,
          })
        }
        state.sealMessage = false
      } else {
        const role = event.role === 'agent' ? 'agent' : 'thought'
        const last = items[items.length - 1]
        if (
          !state.sealMessage &&
          last?.kind === 'thought' &&
          last.role === role
        ) {
          last.text += event.text
          last.seq = seq
        } else {
          items.push({
            kind: 'thought',
            id: `thought-${seq}`,
            role,
            text: event.text,
            seq,
            atMs,
          })
        }
        state.sealMessage = false
      }
      break
    }
    case 'step': {
      const { icon, row, chips } = presentStep(event.step)
      const detail = event.detail || undefined
      if (row === 'divider') {
        flushRun(run, items, state.runSeq)
        items.push({
          kind: 'event',
          id: event.id,
          icon,
          title: event.title,
          detail,
          seq,
          atMs,
        })
        break
      }
      if (!run.length) state.runSeq = seq
      const step: ToolRunStep = {
        kind: 'tool',
        id: event.id,
        icon,
        title: event.title,
        detail,
        chips: state.pendingAutoChip ? [state.pendingAutoChip] : chips,
        tone: 'success',
        quiet: false,
        seq,
        atMs,
        rawDetail: event.detail ?? undefined,
      }
      state.pendingAutoChip = undefined
      stepsById.set(event.id, step)
      run.push(step)
      break
    }
    case 'toolCall': {
      // A question or a plan asking to start shows as its approval card instead.
      if (event.hidden) break
      if (!run.length) state.runSeq = seq
      const scratchRead =
        input.scratch &&
        (event.role.kind === 'read' || event.role.kind === 'search')
      const chips = state.pendingAutoChip
        ? [state.pendingAutoChip]
        : scratchRead
          ? [SCRATCH_CHIP]
          : undefined
      state.pendingAutoChip = undefined
      const step = withRunResult(
        toolStepFromCall(event, seq, atMs, chips),
        event,
        output,
      )
      stepsById.set(event.id, step)
      const parent = event.parent_id
        ? stepsById.get(event.parent_id)
        : undefined
      if (parent?.kind === 'tool') {
        parent.children = [
          ...(parent.children ?? []),
          { ...step, status: event.status },
        ]
        break
      }
      if (step.kind === 'tool' && step.agent) state.agents.push(step)
      run.push(step)
      break
    }
    case 'plan': {
      flushRun(run, items, state.runSeq)
      const steps = event.entries.map((entry) => entry.content)
      const statuses = event.entries.map((entry) => entry.status)
      // An agent re-sends its list as it works: that ticks the turn's plan off, not a new one.
      const lastUser = items.findLastIndex((row) => row.kind === 'user')
      const current = items.findLastIndex((row) => row.kind === 'plan')
      const plan = current > lastUser ? items[current] : undefined
      if (plan?.kind === 'plan') {
        items[current] = { ...plan, steps, statuses }
        break
      }
      items.push({
        kind: 'plan',
        id: `plan-${seq}`,
        steps,
        statuses,
        seq,
        atMs,
      })
      break
    }
    case 'permission': {
      flushRun(run, items, state.runSeq)
      if (event.auto_approved) {
        state.pendingAutoChip = input.scratch
          ? SCRATCH_CHIP
          : { label: 'Read-only, auto-approved', tone: 'muted' }
        break
      }
      items.push(approvalFromPermission(event, seq, atMs, input.approvals))
      break
    }
    case 'currentTool':
    case 'sessionStarted':
    case 'usage':
      flushRun(run, items, state.runSeq)
      state.sealMessage = true
      break
    case 'turnEnd':
    case 'engineExited':
      flushRun(run, items, state.runSeq)
      // Its subagents went with the process, whether or not they reported back.
      if (event.type === 'engineExited') state.agents = []
      if (state.runStartedAt && atMs >= state.runStartedAt) {
        items.push({
          kind: 'toolRun',
          id: `run-worked-${seq}`,
          seq,
          steps: [
            {
              kind: 'tool',
              id: `worked-${seq}`,
              icon: 'clock',
              title: workedForLabel(state.runStartedAt, atMs),
              quiet: true,
              seq,
              atMs,
            },
          ],
        })
      }
      state.runStartedAt = 0
      state.sealMessage = true
      break
  }
}

function finishTimeline(
  state: FoldState,
  input: MapTimelineInput,
): TimelineItem[] {
  flushRun(state.run, state.items, state.runSeq)
  // A thought right before the reply can hold the answer too, so the reply stands alone.
  const items = state.items.filter((item, index) => {
    const next = state.items[index + 1]
    return !(
      item.kind === 'thought' &&
      item.role === 'thought' &&
      next?.kind === 'thought' &&
      next.role === 'agent'
    )
  })

  if (input.findingsReady) {
    items.push({
      kind: 'findings',
      id: 'findings',
      seq: input.events.at(-1)?.seq ?? 0,
    })
  }

  if (input.showLive) {
    // A stable id keeps the row mounted while replies stream, so its animations run on.
    items.push({
      kind: 'live',
      id: 'live',
      agents: runningAgents(state.agents),
    })
  }

  return items
}

export function mapEventsToTimeline(input: MapTimelineInput): TimelineItem[] {
  const state = foldState()
  for (const entry of mergeToolUpdates(input.events)) {
    foldEvent(state, entry, input)
  }
  return finishTimeline(state, input)
}

type FinishedTurns = {
  /** How many of the input's events these items cover. */
  count: number
  first: MapTimelineInput['events'][number] | undefined
  last: MapTimelineInput['events'][number] | undefined
  items: TimelineItem[]
  pendingAutoChip: TimelineChip | undefined
  callIds: Set<string>
  agents: ToolRunStep[]
  approvals: ApprovalRow[]
  scratch: boolean | undefined
}

function sameApprovals(a: ApprovalRow[], b: ApprovalRow[]) {
  if (a === b) return true
  return (
    a.length === b.length &&
    a.every((row, index) => {
      const other = b[index]
      return (
        row.id === other?.id &&
        row.status === other.status &&
        row.payload === other.payload
      )
    })
  )
}

function touchesFinishedTurn(event: TranscriptEvent, callIds: Set<string>) {
  if (event.type === 'toolCallUpdate') return callIds.has(event.id)
  if (event.type === 'step') return callIds.has(event.id)
  if (event.type !== 'toolCall') return false
  return (
    callIds.has(event.id) ||
    (event.parent_id != null && callIds.has(event.parent_id))
  )
}

/**
 * The same timeline as `mapEventsToTimeline`, without redoing finished turns on every event, and
 * handing back the previous object for anything unchanged so its row isn't invalidated. Any event
 * that reaches back into a finished turn, or a change to the approvals, rebuilds from the start.
 */
export class TimelineBuilder {
  private finished: FinishedTurns | null = null
  private previous: TimelineItem[] = []

  private usableTurns(input: MapTimelineInput): FinishedTurns | null {
    const turns = this.finished
    if (
      !turns ||
      turns.scratch !== input.scratch ||
      input.events.length < turns.count ||
      input.events[0] !== turns.first ||
      input.events[turns.count - 1] !== turns.last ||
      !sameApprovals(turns.approvals, input.approvals)
    ) {
      return null
    }
    for (let index = turns.count; index < input.events.length; index += 1) {
      if (touchesFinishedTurn(input.events[index]!.event, turns.callIds)) {
        return null
      }
    }
    return turns
  }

  private fold(input: MapTimelineInput): TimelineItem[] {
    const turns = this.usableTurns(input)
    if (!turns) this.finished = null
    const start = turns?.count ?? 0
    // Sealed: nothing written after a turn ends can extend a message from before it.
    const state = turns
      ? foldState([...turns.items], turns.pendingAutoChip, true, [
          ...turns.agents,
        ])
      : foldState()
    const tail = input.events.slice(start)
    const turnEnd = tail.findLastIndex(
      ({ event }) => event.type === 'turnEnd' || event.type === 'engineExited',
    )
    const boundary = tail[turnEnd]
    for (const entry of mergeToolUpdates(tail)) {
      foldEvent(state, entry, input)
      if (entry !== boundary) continue
      const callIds = new Set(turns?.callIds)
      for (const { event } of tail.slice(0, turnEnd + 1)) {
        if (event.type === 'toolCall' || event.type === 'step') {
          callIds.add(event.id)
        }
      }
      this.finished = {
        count: start + turnEnd + 1,
        first: input.events[0],
        last: boundary,
        items: [...state.items],
        pendingAutoChip: state.pendingAutoChip,
        callIds,
        agents: [...state.agents],
        approvals: input.approvals,
        scratch: input.scratch,
      }
    }
    return finishTimeline(state, input)
  }

  build(input: MapTimelineInput): TimelineItem[] {
    const before = new Map(this.previous.map((item) => [item.id, item]))
    const reused = new Map<TimelineItem, TimelineItem>()
    const items = this.fold(input).map((item) => {
      const old = before.get(item.id)
      if (!old || old === item) return item
      if (!deepEqual(old, item)) return item
      reused.set(item, old)
      return old
    })
    if (reused.size && this.finished) {
      this.finished.items = this.finished.items.map(
        (item) => reused.get(item) ?? item,
      )
    }
    this.previous = items
    return items
  }
}

export function shouldShowLiveRow(status: string, paused: boolean): boolean {
  if (status === 'provisioning' || status === 'running') return true
  if (status === 'paused' && paused) return true
  return false
}

export function liveSubtitle(input: {
  status: string
  paused: boolean
  isLead: boolean
  currentToolTitle: string | null
  activity: string
}): string {
  if (input.paused) return 'Resume to let the agent continue.'
  if (input.status === 'provisioning') {
    return input.isLead ? 'Setting up the worktree' : 'Joining the worktree'
  }
  return input.currentToolTitle ?? input.activity
}

export function liveTitle(input: {
  paused: boolean
  activity: string
}): string {
  return input.paused ? 'Paused by you' : input.activity
}
