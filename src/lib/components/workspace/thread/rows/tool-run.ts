import { plural } from '$lib/format/plural'
import { agentIsRunning, type RunStep } from '$lib/thread/timeline-types'
import type { IconName } from './icon-name'
import { stepIcon } from './step-icons'

type Chip = { label: string; tone: 'success' | 'muted' }

const AUTO_APPROVED: Chip = { label: 'Read-only, auto-approved', tone: 'muted' }
const SCRATCH: Chip = { label: 'Read-only, no worktree', tone: 'muted' }
const FINDINGS_FIXED: Chip[] = [
  { label: 'Tests pass', tone: 'success' },
  { label: 'Committed locally, not pushed', tone: 'muted' },
]

/** A run this long folds down to its summary once the agent has moved on. */
const COLLAPSE_RUN_AT = 6

export function runCollapses(steps: RunStep[], active: boolean): boolean {
  // A run with an agent still working stays open, so the agent stays in view.
  return (
    steps.length >= COLLAPSE_RUN_AT && !active && !steps.some(agentIsRunning)
  )
}

export function toolIcon(step: RunStep): IconName {
  if (step.kind === 'step') return stepIcon(step.step)
  const { role } = step
  switch (role.kind) {
    case 'edit':
      return role.verb === 'Deleted' ? 'trash' : 'pencil'
    case 'read':
      return 'file'
    case 'search':
    case 'toolLookup':
      return 'search'
    // A test run gets a check, the way its summary reads.
    case 'command':
      return role.command.toLowerCase().includes('test') ? 'check' : 'terminal'
    case 'fetch':
    case 'webSearch':
      return 'globe'
    case 'skill':
      return 'skill'
    case 'subagent':
      return 'agent'
    case 'planMode':
    case 'mcp':
    case 'other':
      return 'tool'
  }
}

/** Reads, searches, commands and edits are subtle lines; agents, other tools and core steps stand out. */
export function isQuiet(step: RunStep): boolean {
  if (step.kind === 'step') return false
  return step.role.kind !== 'subagent' && step.role.kind !== 'other'
}

export function stepChips(step: RunStep, scratch: boolean): Chip[] {
  if (step.kind === 'tool' && step.role.kind === 'edit') return []
  // Steps a scratch took, back when scratches ran, are marked as its reads.
  if (step.autoApproved) return [scratch ? SCRATCH : AUTO_APPROVED]
  if (step.kind === 'step') {
    return step.step.kind === 'findingsFixed' ? FINDINGS_FIXED : []
  }
  if (scratch && (step.role.kind === 'read' || step.role.kind === 'search')) {
    return [SCRATCH]
  }
  return []
}

/** A run of reads shares its chips, so only show the ones that change. */
export function newChips(
  step: RunStep,
  prev: RunStep | undefined,
  scratch: boolean,
): Chip[] {
  const prevLabels = prev
    ? stepChips(prev, scratch).map((chip) => chip.label)
    : []
  return stepChips(step, scratch).filter(
    (chip) => !prevLabels.includes(chip.label),
  )
}

type StepSegment = { id: string; quiet: boolean; steps: RunStep[] }

// A run's steps array is replaced only when the run changes, so its segments are kept with it.
const segmentsByRun = new WeakMap<RunStep[], StepSegment[]>()

export function segmentSteps(steps: RunStep[]): StepSegment[] {
  const known = segmentsByRun.get(steps)
  if (known) return known
  const segments: StepSegment[] = []
  for (const step of steps) {
    const quiet = isQuiet(step)
    const last = segments.at(-1)
    if (last?.quiet === quiet) last.steps.push(step)
    else segments.push({ id: step.id, quiet, steps: [step] })
  }
  segmentsByRun.set(steps, segments)
  return segments
}

/** How an agent step is doing: `running · 4 steps`, `4 steps`, `failed`. */
export function agentState(step: RunStep): string {
  if (step.kind !== 'tool' || step.role.kind !== 'subagent') return ''
  const steps = step.children?.length
    ? plural(step.children.length, 'step')
    : ''
  if (step.status === 'failed')
    return ['failed', steps].filter(Boolean).join(' · ')
  if (agentIsRunning(step))
    return ['running', steps].filter(Boolean).join(' · ')
  return steps
}

/** The icon's colour on a step that stands out. */
export function iconTone(step: RunStep): string {
  if (step.kind === 'step') return 'text-success'
  const agent = step.role.kind === 'subagent'
  if (agent && step.status === 'failed') return 'text-destructive'
  if (agentIsRunning(step)) return 'animate-pulse text-primary'
  const settles =
    agent ||
    step.role.kind === 'read' ||
    step.role.kind === 'search' ||
    step.role.kind === 'other'
  return settles && step.status === 'completed'
    ? 'text-success'
    : 'text-primary'
}
