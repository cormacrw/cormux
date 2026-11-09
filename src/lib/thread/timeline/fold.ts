import type {
  ApprovalRow,
  TimelineStep,
  TranscriptEvent,
} from '$lib/ipc/bindings'
import { memoByText } from '$lib/memo'
import { endsTurn } from '../conversations'
import { summarizeRun } from '../reply/blocks/run'
import type {
  RunStep,
  RunningAgent,
  TimelineItem,
  ToolStep,
} from '../timeline-types'
import { agentIsRunning } from '../timeline-types'
import { approvalItem } from './approval'
import { stepLine } from './step-text'

export type FoldInput = {
  /**
   * In store order. `seq` is where the store saved the event, or, for one that is only streamed,
   * where the newest saved event before it is: items take their ids from it, so loading older
   * rows or a reload leaves every id where it was.
   */
  events: { seq: number; atMs?: number; event: TranscriptEvent }[]
  approvals: ApprovalRow[]
}

export type TimelineFold = {
  items: TimelineItem[]
  /** Subagents still working, so they show while the step that spawned them is out of view. */
  agents: RunningAgent[]
}

/** Where a core step goes: between turns, or among the tool steps around it. */
const STEP_ROW: Record<TimelineStep['kind'], 'divider' | 'run'> = {
  agentStopped: 'divider',
  agentPaused: 'divider',
  agentResumed: 'divider',
  sessionStarted: 'divider',
  sessionResumed: 'divider',
  startedByThread: 'divider',
  messageFromThread: 'divider',
  reportFromThread: 'divider',
  fileDeleted: 'divider',
  migrationApplied: 'divider',
  suggestionsForwarded: 'divider',
  suggestionsSent: 'divider',
  branchSwitched: 'divider',
  commitsPulled: 'divider',
  branchRebased: 'divider',
  worktreeCreated: 'divider',
  worktreeReady: 'divider',
  prOpened: 'divider',
  prCheckedOut: 'divider',
  reviewSubmitted: 'divider',
  reviewFindingsUnreadable: 'divider',
  reviewFindingsReady: 'divider',
  findingsReceived: 'run',
  findingsFixed: 'run',
  legacy: 'divider',
}

/** A legacy step (saved by an older build with only its title) says itself whether it joins a run. */
function joinsRun(step: TimelineStep): boolean {
  if (step.kind === 'legacy') return step.run
  return STEP_ROW[step.kind] === 'run'
}

// A rebuild re-reads every command's output, and none changes once written.
const summarizedRun = memoByText(summarizeRun)

type ToolCallEvent = Extract<TranscriptEvent, { type: 'toolCall' }>
type TimelineEvent = FoldInput['events'][number] & { output?: string }

function toolStep(
  event: ToolCallEvent,
  seq: number,
  atMs: number,
  output: string | undefined,
  autoApproved: boolean,
): ToolStep {
  const step: ToolStep = {
    kind: 'tool',
    id: event.id,
    role: event.role,
    title: event.title,
    detail: event.detail,
    locations: event.locations,
    status: event.status,
    seq,
    atMs,
  }
  if (autoApproved) step.autoApproved = true
  if (event.role.kind === 'command' && output) {
    const run = summarizedRun(output)
    if (run) {
      step.run = run
      step.output = output
    }
  }
  return step
}

/** A run takes its id from its first step, whose id is the tool call's own. */
function flushRun(run: RunStep[], items: TimelineItem[], seq: number) {
  const first = run[0]
  if (!first) return
  items.push({ kind: 'toolRun', id: `run-${first.id}`, steps: [...run], seq })
  run.length = 0
}

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
  run: RunStep[]
  // A subagent's steps can land runs after the step that spawned them.
  stepsById: Map<string, ToolStep>
  // A background agent outlives the run that started it.
  agents: ToolStep[]
  runSeq: number
  autoApproved: boolean
  sealMessage: boolean
  // Set when the user sends the prompt; cleared when that turn ends.
  runStartedAt: number
}

export function foldState(
  items: TimelineItem[] = [],
  autoApproved = false,
  sealMessage = false,
  agents: ToolStep[] = [],
): FoldState {
  return {
    items,
    run: [],
    stepsById: new Map(),
    agents,
    runSeq: 0,
    autoApproved,
    sealMessage,
    runStartedAt: 0,
  }
}

function foldMessage(
  state: FoldState,
  kind: 'user' | 'reply' | 'thought',
  text: string,
  seq: number,
  atMs: number,
) {
  const last = state.items.at(-1)
  if (!state.sealMessage && last && 'text' in last && last.kind === kind) {
    last.text += text
    last.seq = seq
  } else {
    const id = `${kind}-${seq}`
    state.items.push({ kind, id, text, seq, atMs })
  }
  state.sealMessage = false
}

function foldEvent(
  state: FoldState,
  { seq, atMs = 0, event, output }: TimelineEvent,
  approvals: ApprovalRow[],
) {
  const { items, run, stepsById } = state
  if (endsTurn(event)) {
    flushRun(run, items, state.runSeq)
    // Its subagents went with the process, whether or not they reported back.
    if (event.type === 'engineExited') state.agents = []
    if (state.runStartedAt && atMs >= state.runStartedAt) {
      items.push({
        kind: 'worked',
        id: `run-worked-${seq}`,
        startedAtMs: state.runStartedAt,
        seq,
        atMs,
      })
    }
    state.runStartedAt = 0
    state.sealMessage = true
    return
  }
  switch (event.type) {
    case 'messageChunk': {
      flushRun(run, items, state.runSeq)
      if (event.role === 'user' && !state.runStartedAt && atMs) {
        state.runStartedAt = atMs
      }
      const kind =
        event.role === 'user'
          ? 'user'
          : event.role === 'agent'
            ? 'reply'
            : 'thought'
      foldMessage(state, kind, event.text, seq, atMs)
      break
    }
    case 'step': {
      if (event.step.kind === 'reviewFindingsReady') {
        flushRun(run, items, state.runSeq)
        items.push({ kind: 'findings', id: event.id, seq })
        break
      }
      if (!joinsRun(event.step)) {
        flushRun(run, items, state.runSeq)
        items.push({
          kind: 'event',
          id: event.id,
          step: event.step,
          title: event.title,
          detail: event.detail,
          seq,
          atMs,
        })
        break
      }
      if (!run.length) state.runSeq = seq
      run.push({
        kind: 'step',
        id: event.id,
        step: event.step,
        title: event.title,
        detail: event.detail,
        seq,
        atMs,
        ...(state.autoApproved && { autoApproved: true }),
      })
      state.autoApproved = false
      break
    }
    case 'toolCall': {
      // A question or a plan asking to start shows as its approval card instead.
      if (event.hidden) break
      if (!run.length) state.runSeq = seq
      const step = toolStep(event, seq, atMs, output, state.autoApproved)
      state.autoApproved = false
      stepsById.set(event.id, step)
      const parent = event.parent_id
        ? stepsById.get(event.parent_id)
        : undefined
      if (parent) {
        parent.children = [...(parent.children ?? []), step]
        break
      }
      if (step.role.kind === 'subagent') state.agents.push(step)
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
        state.autoApproved = true
        break
      }
      items.push(approvalItem(event, seq, atMs, approvals))
      break
    }
    case 'currentTool':
    case 'sessionStarted':
    case 'usage':
      flushRun(run, items, state.runSeq)
      state.sealMessage = true
      break
  }
}

function runningAgents(agents: ToolStep[]): RunningAgent[] {
  return agents.filter(agentIsRunning).map((step) => {
    const children = step.children ?? []
    const last = children.at(-1)
    return {
      id: step.id,
      description: step.detail ?? 'Subagent',
      steps: children.length,
      latest: last ? stepLine(last) : undefined,
    }
  })
}

export function finishFold(state: FoldState): TimelineFold {
  flushRun(state.run, state.items, state.runSeq)
  // A thought right before the reply can hold the answer too, so the reply stands alone.
  const items = state.items.filter(
    (item, index) =>
      !(item.kind === 'thought' && state.items[index + 1]?.kind === 'reply'),
  )
  return { items, agents: runningAgents(state.agents) }
}

export function foldEvents(
  state: FoldState,
  events: TimelineEvent[],
  approvals: ApprovalRow[],
  onEvent?: (entry: TimelineEvent) => void,
) {
  for (const entry of mergeToolUpdates(events)) {
    foldEvent(state, entry, approvals)
    onEvent?.(entry)
  }
}

/** The whole timeline from scratch; `TimelineBuilder` gives the same, incrementally. */
export function foldTimeline(input: FoldInput): TimelineFold {
  const state = foldState()
  foldEvents(state, input.events, input.approvals)
  return finishFold(state)
}
