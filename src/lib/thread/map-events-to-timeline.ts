import type { AgentEvent, ApprovalRow, ToolKind } from '$lib/ipc/bindings'
import { parseApprovalPayload } from '$lib/approvals/payload'
import type {
  TimelineChip,
  TimelineItem,
  ToolRunStep,
  ToolStepIcon,
} from './timeline-types'
import { toolKindIsEdit, toolKindIsRunStep } from './timeline-types'

export type MapTimelineInput = {
  events: { seq: number; atMs?: number; event: AgentEvent }[]
  approvals: ApprovalRow[]
  findingsReady: boolean
  showLive: boolean
  liveToolTitle: string | null
  /** A scratch runs on the main checkout in the engine's read-only mode. */
  scratch?: boolean
}

const SCRATCH_CHIP: TimelineChip = { label: 'Read-only, no worktree', tone: 'muted' }

let syntheticId = 0
function nextId(prefix: string): string {
  syntheticId += 1
  return `${prefix}-${syntheticId}`
}

export function resetTimelineIdCounter() {
  syntheticId = 0
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
    title: payload?.title ?? event.title,
    what: payload?.what ?? event.detail ?? event.tool_name,
    why: payload?.why ?? event.tool_name,
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
    const path = event.locations[0] ?? event.title
    return {
      kind: 'edit',
      id: event.id,
      path,
      verb: editVerb(event.kind, event.title),
      seq,
      atMs,
    }
  }
  const fixedChips = /^Fixed \d+ findings/i.test(event.title)
    ? ([
        { label: 'Tests pass', tone: 'success' as const },
        { label: 'Committed locally, not pushed', tone: 'muted' as const },
      ] as TimelineChip[])
    : undefined
  const tone =
    event.status === 'completed' && event.kind !== 'execute'
      ? ('success' as const)
      : undefined
  const openedPrChips =
    /^Opened PR #\d+/.test(event.title) && !chips?.length
      ? [{ label: 'Checks running' as const }]
      : undefined
  return {
    kind: 'tool',
    id: event.id,
    icon: iconForTool(event.kind, event.title),
    title: event.title,
    detail: (event.detail ?? event.locations.join(', ')) || undefined,
    chips: chips ?? fixedChips ?? openedPrChips,
    tone:
      tone ??
      (/^Opened PR #\d+/.test(event.title) || /^Fixed \d+ findings/i.test(event.title)
        ? 'success'
        : undefined),
    seq,
    atMs,
    rawDetail: event.detail ?? undefined,
  }
}

function flushRun(run: ToolRunStep[], items: TimelineItem[], seq: number) {
  if (!run.length) return
  items.push({
    kind: 'toolRun',
    id: nextId('run'),
    steps: [...run],
    seq,
  })
  run.length = 0
}

function pushBeforeLive(items: TimelineItem[], item: TimelineItem) {
  const liveIdx = items.findIndex((row) => row.kind === 'live')
  if (liveIdx === -1) items.push(item)
  else items.splice(liveIdx, 0, item)
}

export function mapEventsToTimeline(input: MapTimelineInput): TimelineItem[] {
  const items: TimelineItem[] = []
  const run: ToolRunStep[] = []
  let runSeq = 0
  let pendingAutoChip: TimelineChip | undefined
  let sealMessage = false

  for (const { seq, atMs = 0, event } of input.events) {
    switch (event.type) {
      case 'messageChunk': {
        flushRun(run, items, runSeq)
        if (event.role === 'user') {
          const last = items[items.length - 1]
          if (!sealMessage && last?.kind === 'user') {
            last.text += event.text
            last.seq = seq
          } else {
            pushBeforeLive(items, {
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
          if (
            !sealMessage &&
            last?.kind === 'thought' &&
            last.role === role
          ) {
            last.text += event.text
            last.seq = seq
          } else {
            pushBeforeLive(items, {
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
        flushRun(run, items, runSeq)
        pushBeforeLive(items, {
          kind: 'plan',
          id: nextId('plan'),
          steps: event.entries.map((entry) => entry.content),
          seq,
          atMs,
        })
        break
      }
      case 'permission': {
        flushRun(run, items, runSeq)
        if (event.auto_approved) {
          pendingAutoChip = input.scratch
            ? SCRATCH_CHIP
            : { label: 'Read-only, auto-approved', tone: 'muted' }
          break
        }
        pushBeforeLive(
          items,
          approvalFromPermission(event, seq, atMs, input.approvals),
        )
        break
      }
      case 'currentTool':
      case 'sessionStarted':
      case 'usage':
      case 'turnEnd':
      case 'engineExited':
        flushRun(run, items, runSeq)
        sealMessage = true
        break
    }
  }

  flushRun(run, items, runSeq)

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
    pushBeforeLive(items, {
      kind: 'findings',
      id: nextId('findings'),
      seq: input.events.at(-1)?.seq ?? 0,
    })
  }

  if (input.showLive) {
    items.push({ kind: 'live', id: nextId('live') })
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
