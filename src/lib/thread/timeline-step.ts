import type { TimelineStep } from '$lib/ipc/bindings'
import type { TimelineChip, ToolStepIcon } from './timeline-types'

export type StepPresentation = {
  icon: ToolStepIcon
  /** `divider` sits between turns; `run` joins the tool steps around it, with any chips. */
  row: 'divider' | 'run'
  chips?: TimelineChip[]
}

const divider = (icon: ToolStepIcon): StepPresentation => ({
  icon,
  row: 'divider',
})

/**
 * How each kind of step the core writes looks. Its title is display text and plays no part here,
 * so rewording one in `timeline.rs` changes the words and nothing else.
 */
const STEP_PRESENTATION: Record<TimelineStep['kind'], StepPresentation> = {
  agentStopped: divider('stop'),
  agentPaused: divider('pause'),
  agentResumed: divider('play'),
  sessionStarted: divider('session'),
  sessionResumed: divider('tool'),
  startedByThread: divider('tool'),
  messageFromThread: divider('tool'),
  reportFromThread: divider('tool'),
  fileDeleted: divider('trash'),
  migrationApplied: divider('database'),
  suggestionsForwarded: divider('pr'),
  suggestionsSent: divider('pr'),
  branchSwitched: divider('branch'),
  commitsPulled: divider('download'),
  branchRebased: divider('branch'),
  worktreeCreated: divider('branch'),
  worktreeReady: divider('branch'),
  prOpened: divider('pr'),
  prCheckedOut: divider('pr'),
  reviewSubmitted: divider('pr'),
  reviewFindingsUnreadable: divider('tool'),
  findingsReceived: { icon: 'list', row: 'run' },
  findingsFixed: {
    icon: 'check',
    row: 'run',
    chips: [
      { label: 'Tests pass', tone: 'success' },
      { label: 'Committed locally, not pushed', tone: 'muted' },
    ],
  },
}

/** A kind this build doesn't know (the store outlived a downgrade) still shows, plainly. */
export function presentStep(step: TimelineStep): StepPresentation {
  return STEP_PRESENTATION[step.kind] ?? divider('tool')
}
