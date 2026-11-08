import type { TimelineStep } from '$lib/ipc/bindings'
import type { IconName } from './icon-name'

/**
 * Each kind of step the core writes draws with its own icon. Its title is display text and plays
 * no part here, so rewording one in `timeline.rs` changes the words and nothing else.
 */
const STEP_ICONS: Record<TimelineStep['kind'], IconName> = {
  agentStopped: 'stop',
  agentPaused: 'pause',
  agentResumed: 'play',
  sessionStarted: 'session',
  sessionResumed: 'tool',
  startedByThread: 'tool',
  messageFromThread: 'tool',
  reportFromThread: 'tool',
  fileDeleted: 'trash',
  migrationApplied: 'database',
  suggestionsForwarded: 'pr',
  suggestionsSent: 'pr',
  branchSwitched: 'branch',
  commitsPulled: 'download',
  branchRebased: 'branch',
  worktreeCreated: 'branch',
  worktreeReady: 'branch',
  prOpened: 'pr',
  prCheckedOut: 'pr',
  reviewSubmitted: 'pr',
  reviewFindingsUnreadable: 'tool',
  findingsReceived: 'list',
  findingsFixed: 'check',
  legacy: 'tool',
}

/** A kind this build doesn't know (the store outlived a downgrade) still shows, plainly. */
export function stepIcon(step: TimelineStep): IconName {
  if (step.kind === 'legacy') return step.icon
  return STEP_ICONS[step.kind] ?? 'tool'
}
