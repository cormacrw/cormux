import {
  WORKING_THREAD_STATUSES,
  type ReviewRow,
  type ThreadStatus,
  type WorkspaceActivity,
  type WorkspaceLifecycle,
  type WorkspaceRecord,
} from '$lib/ipc/bindings'
import { plural } from '$lib/format/plural'

/** What a thread is doing, in words; `setup` is its workspace's, for a thread still being set up. */
export function threadActivity(
  status: ThreadStatus,
  setup?: WorkspaceActivity,
): string {
  switch (status) {
    case 'running':
      return 'Working'
    case 'paused':
      return 'Paused'
    case 'provisioning':
      return setup?.kind === 'startingAgent'
        ? 'Starting agent…'
        : 'Setting up the worktree'
    default:
      return 'Idle'
  }
}

function setupWords(kind: WorkspaceRecord['kind']): string {
  return kind === 'review' ? 'Checking out the PR…' : 'Running worktree setup…'
}

/**
 * What a workspace is doing, in the words every place that names it uses: a failed setup first,
 * then paused, else what the core says it's at, else its lifecycle. The agent's first read only
 * lasts its first turn. A PR review that isn't working says what its stored `review` came to.
 */
export function workspaceActivity(
  record: Pick<WorkspaceRecord, 'status' | 'kind' | 'activity'>,
  review?: Pick<ReviewRow, 'status' | 'submitted'> | null,
): string {
  const prReview = record.kind === 'review'
  if (record.status === 'provisioningFailed') return 'Setup failed'
  if (record.status === 'paused') return 'Paused'
  switch (record.activity.kind) {
    case 'settingUp':
      return setupWords(record.kind)
    case 'startingAgent':
      return 'Starting agent…'
    case 'agentStarted':
      if (record.status !== 'running') break
      return prReview ? 'Reading the diff…' : 'Reading repo…'
    case 'setupFailed':
      return 'Setup failed'
    case 'none':
      break
  }
  if (record.status === 'provisioning') return setupWords(record.kind)
  if (record.status === 'running') return 'Working'
  if (prReview && review) {
    if (review.submitted) return 'Review submitted'
    if (review.status === 'ready') return 'Review ready'
  }
  return record.status === 'ready' ? 'Ready' : 'Idle'
}

/**
 * A workspace's line in the sidebar: a failed setup, which stays on show (`failed`), then
 * approvals waiting, then what it's doing.
 */
export function workspaceStatusLine(workspace: {
  lifecycle: WorkspaceLifecycle
  pendingApprovals: number
  activityText: string
}): { text: string; failed: boolean } {
  const failed = workspace.lifecycle === 'provisioningFailed'
  if (!failed && workspace.pendingApprovals > 0) {
    return {
      text: `${plural(workspace.pendingApprovals, 'approval', 'approvals')} waiting`,
      failed,
    }
  }
  return { text: workspace.activityText, failed }
}

export function isWorkspaceProvisioning(
  lifecycle: WorkspaceLifecycle,
): boolean {
  return lifecycle === 'provisioning'
}

/** An agent working or starting, by the core's rule. */
export function isAgentWorking(status: ThreadStatus): boolean {
  return (WORKING_THREAD_STATUSES as readonly ThreadStatus[]).includes(status)
}

export function runningAgentCount(threads: { status: ThreadStatus }[]): number {
  return threads.filter((thread) => isAgentWorking(thread.status)).length
}
