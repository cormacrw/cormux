import type { ToolKind } from '$lib/ipc/bindings'

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

export type ToolRunStep =
  | {
      kind: 'tool'
      id: string
      icon: ToolStepIcon
      title: string
      detail?: string
      chips?: TimelineChip[]
      tone?: 'success'
      seq: number
      rawDetail?: string
    }
  | {
      kind: 'edit'
      id: string
      path: string
      verb: 'Edited' | 'Created' | 'Deleted'
      seq: number
    }

export type TimelineItem =
  | { kind: 'user'; id: string; text: string; seq: number }
  | {
      kind: 'thought'
      id: string
      text: string
      seq: number
      streaming?: boolean
    }
  | { kind: 'toolRun'; id: string; steps: ToolRunStep[]; seq: number }
  | { kind: 'plan'; id: string; steps: string[]; seq: number }
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
      seq: number
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
