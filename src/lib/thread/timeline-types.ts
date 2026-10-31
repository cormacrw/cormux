import type { ToolCallStatus, ToolKind } from '$lib/ipc/bindings'
import type { AgentQuestion } from '$lib/approvals/payload'
import type { RunSummary } from './run-summary'

export type ChipTone = 'success' | 'warning' | 'muted'

export type TimelineChip = {
  label: string
  tone?: ChipTone
}

export type ToolStepIcon =
  | 'file'
  | 'tool'
  | 'branch'
  | 'check'
  | 'terminal'
  | 'database'
  | 'pr'
  | 'download'
  | 'list'
  | 'search'
  | 'pencil'
  | 'trash'
  | 'pause'
  | 'play'
  | 'stop'
  | 'session'
  | 'globe'
  | 'skill'
  | 'clock'

export type ToolRunStep =
  | {
      kind: 'tool'
      id: string
      icon: ToolStepIcon
      title: string
      detail?: string
      chips?: TimelineChip[]
      tone?: 'success'
      /** Reads and searches render as a subtle line instead of a card row. */
      quiet?: boolean
      seq: number
      atMs: number
      rawDetail?: string
      /** What a test or build command came to, and the end of what it printed. */
      run?: RunSummary
      output?: string
      /** What a subagent did, when this step spawned one. */
      children?: ToolRunStep[]
      status?: ToolCallStatus
    }
  | {
      kind: 'edit'
      id: string
      path: string
      verb: 'Edited' | 'Created' | 'Deleted'
      seq: number
      atMs: number
      status?: ToolCallStatus
    }

export type TimelineItem =
  | { kind: 'user'; id: string; text: string; seq: number; atMs: number }
  | {
      kind: 'thought'
      id: string
      text: string
      seq: number
      atMs: number
      role: 'agent' | 'thought'
      streaming?: boolean
    }
  | { kind: 'toolRun'; id: string; steps: ToolRunStep[]; seq: number }
  | {
      kind: 'plan'
      id: string
      steps: string[]
      /** Each step's status, as the engine reports it: `pending`, `in_progress`, `completed`. */
      statuses: string[]
      seq: number
      atMs: number
    }
  | {
      kind: 'approval'
      id: string
      title: string
      what: string
      why: string
      okLabel: string
      noLabel: string
      state: 'pending' | 'approved' | 'denied'
      doneAtMs: number | null
      /** Set when the agent is asking something rather than seeking permission. */
      questions: AgentQuestion[]
      answers: Record<string, string>
      seq: number
      atMs: number
    }
  | {
      /** Something the app did in the thread (switched branch, pulled, opened a PR), not the agent. */
      kind: 'event'
      id: string
      icon: ToolStepIcon
      title: string
      detail?: string
      seq: number
      atMs: number
    }
  | { kind: 'findings'; id: string; seq: number }
  | { kind: 'live'; id: string }

export type TimelineSpeaker = 'user' | 'agent'

export type TimelineRow =
  | {
      kind: 'speaker'
      id: string
      speaker: TimelineSpeaker
      role: string
      engine: string
      seq: number
      atMs: number
    }
  | { kind: 'item'; item: TimelineItem }

export function toolKindIsEdit(kind: ToolKind): boolean {
  return kind === 'edit' || kind === 'delete' || kind === 'move'
}

export function toolKindIsRunStep(kind: ToolKind): boolean {
  if (toolKindIsEdit(kind)) return true
  return (
    kind === 'read' ||
    kind === 'search' ||
    kind === 'execute' ||
    kind === 'fetch' ||
    kind === 'other' ||
    kind === 'think'
  )
}
