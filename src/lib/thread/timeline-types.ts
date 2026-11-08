import type { TimelineStep, ToolCallStatus, ToolRole } from '$lib/ipc/bindings'
import type { AgentQuestion } from '$lib/approvals/payload'
import type { RunSummary } from './reply/blocks/run'

/** A tool call the agent made, with whatever its later updates filled in. */
export type ToolStep = {
  kind: 'tool'
  id: string
  role: ToolRole
  /** The engine's own words for the call; most roles are named by the row instead. */
  title: string
  detail: string | null
  locations: string[]
  status: ToolCallStatus
  seq: number
  atMs: number
  /** Ran without asking, because the call only reads. */
  autoApproved?: true
  /** What a test or build command came to, and the end of what it printed. */
  run?: RunSummary
  output?: string
  /** What a subagent did, when this call spawned one. */
  children?: ToolStep[]
}

/** Something the core did that sits among the agent's tool steps (findings sent or fixed). */
type CoreStep = {
  kind: 'step'
  id: string
  step: TimelineStep
  title: string
  detail: string | null
  seq: number
  atMs: number
  autoApproved?: true
}

export type RunStep = ToolStep | CoreStep

export type TimelineItem =
  | { kind: 'user'; id: string; text: string; seq: number; atMs: number }
  | { kind: 'reply'; id: string; text: string; seq: number; atMs: number }
  | { kind: 'thought'; id: string; text: string; seq: number; atMs: number }
  | { kind: 'toolRun'; id: string; steps: RunStep[]; seq: number }
  | {
      /** A turn's end, with when it started. */
      kind: 'worked'
      id: string
      startedAtMs: number
      seq: number
      atMs: number
    }
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
      answers: Partial<Record<string, string>>
      seq: number
      atMs: number
    }
  | {
      /** Something the app did in the thread (switched branch, pulled, opened a PR), not the agent. */
      kind: 'event'
      id: string
      step: TimelineStep
      title: string
      detail: string | null
      seq: number
      atMs: number
    }
  | {
      /** The workspace's review finished with findings to look at. */
      kind: 'findings'
      id: string
      workspaceId: string
      seq: number
    }

/** A subagent still working, with the last thing it did. */
export type RunningAgent = {
  id: string
  description: string
  steps: number
  latest?: string
}

export function agentIsRunning(step: RunStep): boolean {
  return (
    step.kind === 'tool' &&
    step.role.kind === 'subagent' &&
    (step.status === 'inProgress' || step.status === 'pending')
  )
}

export type TimelineRow =
  | {
      kind: 'speaker'
      id: string
      role: string
      engine: string
      seq: number
      atMs: number
    }
  | { kind: 'item'; item: TimelineItem }
