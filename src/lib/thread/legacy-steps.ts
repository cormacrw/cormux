/**
 * Everything that reads meaning from a step's title. Rows the core saved before it wrote typed
 * steps (`AgentEvent`'s `step`, see `timeline-step.ts`) have no kind, so their English is all
 * there is; they stay in users' databases and must keep rendering as they did.
 *
 * Nothing the core writes now arrives here, so these titles can't go stale, and none of this
 * should grow. Rows come in three forms: a `control-`, `review-` or `pr-open-` tool call; a
 * `finding-` tool call; and a bare `{icon, title, detail}`, whose `icon` was never read.
 */
import type { AgentEvent } from '$lib/ipc/bindings'
import type { TimelineChip, ToolStepIcon } from './timeline-types'

type ToolCall = Extract<AgentEvent, { type: 'toolCall' }>

export const NEW_SESSION_TITLE = 'Started a new session'
export const RESUME_TITLE = 'Resumed an earlier conversation'

/** A bare `{icon, title, detail}` row, as a tool call the rest of the timeline can read. */
export function normalizeLegacyStep(
  raw: Record<string, unknown>,
  seq: number | undefined,
): AgentEvent | null {
  if (typeof raw.title !== 'string') return null
  return {
    type: 'toolCall',
    // Titles repeat (switching back to a branch), and a repeated key crashes the timeline.
    id: seq === undefined ? `legacy-${raw.title}` : `legacy-${seq}`,
    title: raw.title,
    name: null,
    kind: 'other',
    status: 'completed',
    locations: [],
    detail: typeof raw.detail === 'string' ? raw.detail : null,
  }
}

// `finding-` rows were left out, so those drew as tool steps rather than dividers.
const LEGACY_DIVIDER_ID = /^(control|legacy|pr-open|scratch-open|review)-/

export function isLegacyDivider(event: ToolCall): boolean {
  return LEGACY_DIVIDER_ID.test(event.id)
}

export function legacyDividerIcon(title: string): ToolStepIcon {
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

/*
 * The rest also runs on the agent's own tool calls, which share the path `finding-` rows take. A
 * tool call can't say whether it's a saved core step, so these stay title-only, and an agent tool
 * that happens to be titled this way keeps looking the way it always has.
 */

export function legacyToolIcon(title: string): ToolStepIcon | null {
  const lower = title.toLowerCase()
  if (/received \d+ findings from the review/i.test(title)) return 'list'
  if (/^fixed \d+ findings/i.test(title)) return 'check'
  if (lower.includes('you paused')) return 'pause'
  if (lower.includes('you resumed')) return 'play'
  return null
}

export function legacyToolChips(title: string): TimelineChip[] | undefined {
  if (/^Fixed \d+ findings/i.test(title)) {
    return [
      { label: 'Tests pass', tone: 'success' },
      { label: 'Committed locally, not pushed', tone: 'muted' },
    ]
  }
  if (/^Opened PR #\d+/.test(title)) return [{ label: 'Checks running' }]
  return undefined
}

export function legacyToolSucceeded(title: string): boolean {
  return /^Opened PR #\d+/.test(title) || /^Fixed \d+ findings/i.test(title)
}

/** The boundary a saved `control-` step marks, for splitting a thread into conversations. */
export function legacySessionBoundary(
  event: AgentEvent,
): { ended: unknown; conversation: unknown } | null {
  if (event.type !== 'toolCall' || !event.id.startsWith('control-')) return null
  const starting = event.title === NEW_SESSION_TITLE
  if (!starting && event.title !== RESUME_TITLE) return null
  const detail = jsonDetail(event.detail)
  return {
    ended: detail?.endedSession,
    conversation: starting ? null : detail?.conversation,
  }
}

function jsonDetail(detail: string | null): Record<string, unknown> | null {
  if (!detail) return null
  try {
    const parsed: unknown = JSON.parse(detail)
    return parsed && typeof parsed === 'object'
      ? (parsed as Record<string, unknown>)
      : null
  } catch {
    return null
  }
}

export function isLegacyNewSession(event: AgentEvent): boolean {
  return event.type === 'toolCall' && event.title === NEW_SESSION_TITLE
}
