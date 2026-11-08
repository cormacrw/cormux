import type { AgentEvent, ApprovalRow, ToolKind } from '$lib/ipc/bindings'
import { parseApprovalPayload } from '$lib/approvals/payload'
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
import { agentIsRunning, isAgentTool, toolKindIsEdit } from './timeline-types'
import {
  isLegacyDivider,
  legacyDividerIcon,
  legacyToolChips,
  legacyToolIcon,
  legacyToolSucceeded,
} from './legacy-steps'
import { presentStep } from './timeline-step'
import {
  humanizeIdentifier,
  parseMcpToolName,
  toolDisplayName,
} from './humanize'

export type MapTimelineInput = {
  events: { seq: number; atMs?: number; event: AgentEvent }[]
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

type NextId = (prefix: string) => string

/**
 * Ids count items of each kind (the 3rd user message is `user-3`), so a rebuild after a
 * new chunk or a reload from the store gives every row the same key and it stays mounted.
 */
function ordinalIds(counts: Map<string, number>): NextId {
  return (prefix) => {
    const next = (counts.get(prefix) ?? 0) + 1
    counts.set(prefix, next)
    return `${prefix}-${next}`
  }
}

function iconForTool(kind: ToolKind, title: string): ToolStepIcon {
  const legacy = legacyToolIcon(title)
  if (legacy) return legacy
  const lower = title.toLowerCase()
  if (kind === 'read') return 'file'
  if (kind === 'search') return 'search'
  if (kind === 'execute') return lower.includes('test') ? 'check' : 'terminal'
  if (kind === 'fetch') return 'download'
  if (kind === 'delete') return 'trash'
  if (kind === 'edit' || kind === 'move') return 'pencil'
  if (lower.includes('branch')) return 'branch'
  if (lower.includes('pull') || lower.includes('pr')) return 'pr'
  if (lower.includes('database') || lower.includes('migration'))
    return 'database'
  return 'tool'
}

function editVerb(
  kind: ToolKind,
  title: string,
): 'Edited' | 'Created' | 'Deleted' {
  if (kind === 'delete') return 'Deleted'
  if (/creat/i.test(title)) return 'Created'
  return 'Edited'
}

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

// Approval labels quote the raw tool name ("Use mcp__github__get_pr"); show it as words.
function withToolName(text: string, toolName: string): string {
  return toolName ? text.replace(toolName, toolDisplayName(toolName)) : text
}

// A timeline rebuild re-reads every approval and run, and neither changes once written.
const parsedApproval = memoByText(parseApprovalPayload)
const summarizedRun = memoByText(summarizeRun)

function approvalFromPermission(
  event: Extract<AgentEvent, { type: 'permission' }>,
  seq: number,
  atMs: number,
  approvals: ApprovalRow[],
) {
  const row = approvalById(approvals, event.id)
  const payload = row ? parsedApproval(row.payload) : null
  const state = approvalState(row)
  return {
    kind: 'approval' as const,
    id: event.id,
    title: withToolName(payload?.title ?? event.title, event.tool_name),
    what: withToolName(
      payload?.what ?? event.detail ?? event.tool_name,
      event.tool_name,
    ),
    why: withToolName(payload?.why ?? event.tool_name, event.tool_name),
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
  event: Extract<AgentEvent, { type: 'toolCall' }>,
  seq: number,
  atMs: number,
  chips: TimelineChip[] | undefined,
): ToolRunStep {
  if (toolKindIsEdit(event.kind)) {
    const path = event.locations[0] ?? ''
    return {
      kind: 'edit',
      id: event.id,
      path,
      verb: editVerb(event.kind, event.title),
      seq,
      atMs,
    }
  }
  if (isAgentTool(event.name ?? event.title)) {
    return agentStep(event, seq, atMs, chips)
  }
  if (event.kind === 'execute') return commandStep(event, seq, atMs, chips)
  const named = namedToolStep(event)
  if (named) {
    return {
      kind: 'tool',
      id: event.id,
      ...named,
      chips,
      quiet: true,
      seq,
      atMs,
      rawDetail: event.detail ?? undefined,
    }
  }
  const succeeded =
    event.status === 'completed' || legacyToolSucceeded(event.title)
  return {
    kind: 'tool',
    id: event.id,
    icon: iconForTool(event.kind, event.title),
    // Claude titles these by tool name (`TodoWrite`); show it as words.
    title: humanizeIdentifier(event.title),
    detail: (event.detail ?? event.locations.join(', ')) || undefined,
    chips: chips ?? legacyToolChips(event.title),
    tone: succeeded ? 'success' : undefined,
    quiet: event.kind === 'read' || event.kind === 'search',
    seq,
    atMs,
    rawDetail: event.detail ?? undefined,
  }
}

// Claude's Agent call is detailed by the task's short description.
function agentStep(
  event: Extract<AgentEvent, { type: 'toolCall' }>,
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

// MCP tools arrive as `mcp__<server>__<tool>`, ToolSearch is Claude loading tool schemas,
// and web and skill calls carry their point in the detail; all read better as a quiet
// line than as a raw-name card.
function namedToolStep(
  event: Extract<AgentEvent, { type: 'toolCall' }>,
): { icon: ToolStepIcon; title: string; detail: string | undefined } | null {
  const name = event.name ?? event.title
  if (name === 'WebFetch') {
    return { icon: 'globe', title: 'Fetched', detail: shortUrl(event.detail) }
  }
  if (name === 'WebSearch') {
    return {
      icon: 'globe',
      title: 'Searched the web',
      detail: event.detail ?? undefined,
    }
  }
  if (name === 'Skill') {
    return {
      icon: 'skill',
      title: 'Used skill',
      detail: event.detail ?? undefined,
    }
  }
  if (name === 'EnterPlanMode') {
    return { icon: 'tool', title: 'Switched to plan mode', detail: undefined }
  }
  if (name === 'ToolSearch') {
    return {
      icon: 'search',
      title: 'Looked up tools',
      detail: event.detail ?? undefined,
    }
  }
  const mcp = parseMcpToolName(name)
  if (!mcp) return null
  return {
    icon: 'tool',
    title: `Used ${mcp.server}`,
    detail: event.detail ? `${mcp.action} · ${event.detail}` : mcp.action,
  }
}

const COMMAND_PREVIEW_CHARS = 50

function withRunResult(
  step: ToolRunStep,
  event: Extract<AgentEvent, { type: 'toolCall' }>,
  output: string | undefined,
): ToolRunStep {
  if (step.kind !== 'tool' || event.kind !== 'execute' || !output) return step
  const run = summarizedRun(output)
  return run ? { ...step, run, output } : step
}

// A shell step's title is its command, unless it only repeats the tool's name.
function commandText(event: Extract<AgentEvent, { type: 'toolCall' }>): string {
  const title = event.title.trim()
  if (event.name && title === event.name) return ''
  return title
}

function commandStep(
  event: Extract<AgentEvent, { type: 'toolCall' }>,
  seq: number,
  atMs: number,
  chips: TimelineChip[] | undefined,
): ToolRunStep {
  const command = commandText(event)
  const preview =
    command.length > COMMAND_PREVIEW_CHARS
      ? `${command.slice(0, COMMAND_PREVIEW_CHARS).trimEnd()}…`
      : command
  return {
    kind: 'tool',
    id: event.id,
    icon: iconForTool(event.kind, command),
    title: command ? 'Ran' : 'Ran a command',
    detail: preview || undefined,
    chips,
    quiet: true,
    seq,
    atMs,
    rawDetail: command || undefined,
  }
}

function flushRun(
  run: ToolRunStep[],
  items: TimelineItem[],
  seq: number,
  nextId: NextId,
) {
  if (!run.length) return
  items.push({
    kind: 'toolRun',
    id: nextId('run'),
    steps: [...run],
    seq,
  })
  run.length = 0
}

type TimelineEvent = MapTimelineInput['events'][number] & { output?: string }

// Engines fill in a tool call after announcing it,
// so fold each update into the call it belongs to.
// A repeated id is the same call or step again (a replay stored by an older
// build); keep the first so step ids, which key the rendered list, stay unique.
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
        kind: event.kind ?? call.event.kind,
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
  counts: Map<string, number>
  nextId: NextId
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
  counts = new Map<string, number>(),
  pendingAutoChip?: TimelineChip,
  sealMessage = false,
  agents: ToolRunStep[] = [],
): FoldState {
  return {
    items,
    counts,
    nextId: ordinalIds(counts),
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
  const { items, run, nextId, stepsById } = state
  switch (event.type) {
    case 'messageChunk': {
      flushRun(run, items, state.runSeq, nextId)
      if (event.role === 'user') {
        if (!state.runStartedAt && atMs) state.runStartedAt = atMs
        const last = items[items.length - 1]
        if (!state.sealMessage && last?.kind === 'user') {
          last.text += event.text
          last.seq = seq
        } else {
          items.push({
            kind: 'user',
            id: nextId('user'),
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
            id: nextId('thought'),
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
        flushRun(run, items, state.runSeq, nextId)
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
      if (isLegacyDivider(event)) {
        flushRun(run, items, state.runSeq, nextId)
        items.push({
          kind: 'event',
          id: event.id,
          icon: legacyDividerIcon(event.title),
          title: event.title,
          detail: event.detail ?? undefined,
          seq,
          atMs,
        })
        break
      }
      // The plan's own row stands for the request to leave plan mode.
      const toolName = event.name ?? event.title
      // Both show up as their approval card, never as a step.
      if (toolName === 'ExitPlanMode' || toolName === 'AskUserQuestion') break
      if (!run.length) state.runSeq = seq
      const scratchRead =
        input.scratch && (event.kind === 'read' || event.kind === 'search')
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
      flushRun(run, items, state.runSeq, nextId)
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
        id: nextId('plan'),
        steps,
        statuses,
        seq,
        atMs,
      })
      break
    }
    case 'permission': {
      flushRun(run, items, state.runSeq, nextId)
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
      flushRun(run, items, state.runSeq, nextId)
      state.sealMessage = true
      break
    case 'turnEnd':
    case 'engineExited':
      flushRun(run, items, state.runSeq, nextId)
      // Its subagents went with the process, whether or not they reported back.
      if (event.type === 'engineExited') state.agents = []
      if (state.runStartedAt && atMs >= state.runStartedAt) {
        items.push({
          kind: 'toolRun',
          id: nextId('run'),
          seq,
          steps: [
            {
              kind: 'tool',
              id: nextId('worked'),
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
  flushRun(state.run, state.items, state.runSeq, state.nextId)
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
  counts: Map<string, number>
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

function touchesFinishedTurn(event: AgentEvent, callIds: Set<string>) {
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
      ? foldState(
          [...turns.items],
          new Map(turns.counts),
          turns.pendingAutoChip,
          true,
          [...turns.agents],
        )
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
        counts: new Map(state.counts),
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
