import type { AgentEvent, ApprovalRow, ToolKind } from '$lib/ipc/bindings'
import { parseApprovalPayload } from '$lib/approvals/payload'
import type {
  TimelineChip,
  TimelineItem,
  ToolRunStep,
  ToolStepIcon,
} from './timeline-types'
import { toolKindIsEdit, toolKindIsRunStep } from './timeline-types'
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
  liveToolTitle: string | null
  /** A scratch runs on the main checkout in the engine's read-only mode. */
  scratch?: boolean
}

const SCRATCH_CHIP: TimelineChip = {
  label: 'Read-only, no worktree',
  tone: 'muted',
}

type NextId = (prefix: string) => string

/**
 * Ids count items of each kind (the 3rd user message is `user-3`), so a rebuild after a
 * new chunk or a reload from the store gives every row the same key and it stays mounted.
 */
function ordinalIds(): NextId {
  const counts = new Map<string, number>()
  return (prefix) => {
    const next = (counts.get(prefix) ?? 0) + 1
    counts.set(prefix, next)
    return `${prefix}-${next}`
  }
}

function iconForTool(kind: ToolKind, title: string): ToolStepIcon {
  const lower = title.toLowerCase()
  if (/received \d+ findings from the review/i.test(title)) return 'list'
  if (/^fixed \d+ findings/i.test(title)) return 'check'
  if (lower.includes('you paused')) return 'pause'
  if (lower.includes('you resumed')) return 'play'
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

// The core writes its own steps with these id prefixes; agent tool calls never use them.
const APP_EVENT_ID = /^(control|legacy|pr-open|scratch-open|review)-/

function isAppEvent(event: Extract<AgentEvent, { type: 'toolCall' }>): boolean {
  return APP_EVENT_ID.test(event.id)
}

function iconForAppEvent(title: string): ToolStepIcon {
  const lower = title.toLowerCase()
  if (lower.includes('you paused')) return 'pause'
  if (lower.includes('you resumed')) return 'play'
  if (lower.includes('you stopped')) return 'stop'
  if (lower.includes('new session')) return 'session'
  if (/^pulled /i.test(title)) return 'download'
  if (/^(switched|rebased)|worktree/i.test(title)) return 'branch'
  if (/#\d+|suggestions/i.test(title)) return 'pr'
  if (/^read \d+ changed/i.test(title)) return 'file'
  if (/^deleted /i.test(title)) return 'trash'
  if (lower.includes('migration')) return 'database'
  if (lower.includes('approved')) return 'check'
  if (/^opened /i.test(title)) return 'file'
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
  id: string,
  approvals: ApprovalRow[],
): 'pending' | 'approved' | 'denied' {
  const row = approvals.find((entry) => entry.id === id)
  if (!row) return 'pending'
  if (row.status === 'approved') return 'approved'
  if (row.status === 'denied') return 'denied'
  return 'pending'
}

// Approval labels quote the raw tool name ("Use mcp__github__get_pr"); show it as words.
function withToolName(text: string, toolName: string): string {
  return toolName ? text.replace(toolName, toolDisplayName(toolName)) : text
}

function approvalFromPermission(
  event: Extract<AgentEvent, { type: 'permission' }>,
  seq: number,
  atMs: number,
  approvals: ApprovalRow[],
) {
  const row = approvals.find((entry) => entry.id === event.id)
  const payload = row ? parseApprovalPayload(row.payload) : null
  const state = approvalState(event.id, approvals)
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
  const fixedChips = /^Fixed \d+ findings/i.test(event.title)
    ? ([
        { label: 'Tests pass', tone: 'success' as const },
        { label: 'Committed locally, not pushed', tone: 'muted' as const },
      ] as TimelineChip[])
    : undefined
  const tone = event.status === 'completed' ? ('success' as const) : undefined
  const openedPrChips =
    /^Opened PR #\d+/.test(event.title) && !chips?.length
      ? [{ label: 'Checks running' as const }]
      : undefined
  return {
    kind: 'tool',
    id: event.id,
    icon: iconForTool(event.kind, event.title),
    // Claude titles these by tool name (`TodoWrite`); show it as words.
    title: humanizeIdentifier(event.title),
    detail: (event.detail ?? event.locations.join(', ')) || undefined,
    chips: chips ?? fixedChips ?? openedPrChips,
    tone:
      tone ??
      (/^Opened PR #\d+/.test(event.title) ||
      /^Fixed \d+ findings/i.test(event.title)
        ? 'success'
        : undefined),
    quiet: event.kind === 'read' || event.kind === 'search',
    seq,
    atMs,
    rawDetail: event.detail ?? undefined,
  }
}

// MCP tools arrive as `mcp__<server>__<tool>`, and ToolSearch is Claude loading tool
// schemas; both read better as a quiet line than as a raw-name card.
function namedToolStep(
  event: Extract<AgentEvent, { type: 'toolCall' }>,
): { icon: ToolStepIcon; title: string; detail: string | undefined } | null {
  const name = event.name ?? event.title
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

// Cursor titles a shell step with the whole command in backticks; Claude titles it `Bash`.
function commandText(event: Extract<AgentEvent, { type: 'toolCall' }>): string {
  const title = event.title.trim()
  if (event.name && title === event.name) return ''
  return title.replace(/^`+|`+$/g, '').trim()
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

type TimelineEvent = MapTimelineInput['events'][number]

// Engines fill in a tool call after announcing it (Cursor sends the edit path later),
// so fold each update into the call it belongs to.
function mergeToolUpdates(events: TimelineEvent[]): TimelineEvent[] {
  const merged: TimelineEvent[] = []
  const callIndex = new Map<string, number>()
  for (const entry of events) {
    const { event } = entry
    if (event.type === 'toolCall') {
      // A repeated id is the same call again (a session/load replay stored before that was
      // fixed); keep the first so step ids, which key the rendered list, stay unique.
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

export function mapEventsToTimeline(input: MapTimelineInput): TimelineItem[] {
  const items: TimelineItem[] = []
  const nextId = ordinalIds()
  const run: ToolRunStep[] = []
  let runSeq = 0
  let pendingAutoChip: TimelineChip | undefined
  let sealMessage = false

  for (const { seq, atMs = 0, event } of mergeToolUpdates(input.events)) {
    switch (event.type) {
      case 'messageChunk': {
        flushRun(run, items, runSeq, nextId)
        if (event.role === 'user') {
          const last = items[items.length - 1]
          if (!sealMessage && last?.kind === 'user') {
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
          sealMessage = false
        } else {
          const role = event.role === 'agent' ? 'agent' : 'thought'
          const last = items[items.length - 1]
          if (!sealMessage && last?.kind === 'thought' && last.role === role) {
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
          sealMessage = false
        }
        break
      }
      case 'toolCall': {
        if (isAppEvent(event)) {
          flushRun(run, items, runSeq, nextId)
          items.push({
            kind: 'event',
            id: event.id,
            icon: iconForAppEvent(event.title),
            title: event.title,
            detail: event.detail ?? undefined,
            seq,
            atMs,
          })
          break
        }
        if (!toolKindIsRunStep(event.kind)) break
        if (!run.length) runSeq = seq
        const scratchRead =
          input.scratch && (event.kind === 'read' || event.kind === 'search')
        const chips = pendingAutoChip
          ? [pendingAutoChip]
          : scratchRead
            ? [SCRATCH_CHIP]
            : undefined
        pendingAutoChip = undefined
        run.push(toolStepFromCall(event, seq, atMs, chips))
        break
      }
      case 'plan': {
        flushRun(run, items, runSeq, nextId)
        items.push({
          kind: 'plan',
          id: nextId('plan'),
          steps: event.entries.map((entry) => entry.content),
          seq,
          atMs,
        })
        break
      }
      case 'permission': {
        flushRun(run, items, runSeq, nextId)
        if (event.auto_approved) {
          pendingAutoChip = input.scratch
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
      case 'turnEnd':
      case 'engineExited':
        flushRun(run, items, runSeq, nextId)
        sealMessage = true
        break
    }
  }

  flushRun(run, items, runSeq, nextId)

  // Cursor streams the answer inside the thought, then again as the reply.
  const visible = items.filter((item, index) => {
    const next = items[index + 1]
    return !(
      item.kind === 'thought' &&
      item.role === 'thought' &&
      next?.kind === 'thought' &&
      next.role === 'agent'
    )
  })
  items.length = 0
  items.push(...visible)

  if (input.findingsReady) {
    items.push({
      kind: 'findings',
      id: 'findings',
      seq: input.events.at(-1)?.seq ?? 0,
    })
  }

  if (input.showLive) {
    // A stable id keeps the row mounted while replies stream, so its animations run on.
    items.push({ kind: 'live', id: 'live' })
  }

  void input.liveToolTitle
  return items
}

export function applyAgentEventToTimeline(
  items: TimelineItem[],
  event: AgentEvent,
  seq: number,
  approvals: ApprovalRow[],
): TimelineItem[] {
  return mapEventsToTimeline({
    events: flattenItemsToEvents(items, event, seq),
    approvals,
    findingsReady: items.some((row) => row.kind === 'findings'),
    showLive: items.some((row) => row.kind === 'live'),
    liveToolTitle: null,
  })
}

function flattenItemsToEvents(
  items: TimelineItem[],
  nextEvent: AgentEvent,
  seq: number,
): { seq: number; event: AgentEvent }[] {
  const events: { seq: number; event: AgentEvent }[] = []
  for (const item of items) {
    if (item.kind === 'user') {
      events.push({
        seq: item.seq,
        event: { type: 'messageChunk', role: 'user', text: item.text },
      })
    } else if (item.kind === 'thought') {
      events.push({
        seq: item.seq,
        event: {
          type: 'messageChunk',
          role: item.role,
          text: item.text,
        },
      })
    } else if (item.kind === 'toolRun') {
      for (const step of item.steps) {
        if (step.kind === 'edit') {
          events.push({
            seq: step.seq,
            event: {
              type: 'toolCall',
              id: step.id,
              title: step.verb,
              name: null,
              kind: step.verb === 'Deleted' ? 'delete' : 'edit',
              status: 'completed',
              locations: [step.path],
              detail: step.path,
            },
          })
        } else {
          events.push({
            seq: step.seq,
            event: {
              type: 'toolCall',
              id: step.id,
              title: step.title,
              name: null,
              kind: 'other',
              status: 'completed',
              locations: step.detail ? [step.detail] : [],
              detail: step.rawDetail ?? step.detail ?? null,
            },
          })
        }
      }
    } else if (item.kind === 'event') {
      events.push({
        seq: item.seq,
        event: {
          type: 'toolCall',
          id: item.id,
          title: item.title,
          name: null,
          kind: 'other',
          status: 'completed',
          locations: [],
          detail: item.detail ?? null,
        },
      })
    } else if (item.kind === 'plan') {
      events.push({
        seq: item.seq,
        event: {
          type: 'plan',
          entries: item.steps.map((content) => ({
            content,
            status: 'pending',
          })),
        },
      })
    } else if (item.kind === 'approval' && item.state === 'pending') {
      events.push({
        seq: item.seq,
        event: {
          type: 'permission',
          id: item.id,
          tool_call_id: null,
          title: item.title,
          tool_name: item.why,
          kind: 'other',
          detail: item.what,
          auto_approved: false,
        },
      })
    }
  }
  events.push({ seq, event: nextEvent })
  return events
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
  status: string
  paused: boolean
  activity: string
}): string {
  if (input.paused) return 'Paused by you'
  if (input.status === 'provisioning') return input.activity
  return input.activity
}
