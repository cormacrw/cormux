import {
  WORKING_THREAD_STATUSES,
  type ThreadStatus,
  type WorkspaceActivity,
  type WorkspaceLifecycle,
  type WorkspaceRecord,
} from '$lib/ipc/bindings'

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

/**
 * What a workspace is doing, in the words every place that names it uses: paused if it is, else
 * what the core says it's at, else its lifecycle. The agent's first read only lasts its first turn.
 */
export function workspaceActivity(
  record: Pick<WorkspaceRecord, 'status' | 'kind' | 'activity'>,
): string {
  const review = record.kind === 'review'
  if (record.status === 'paused') return 'Paused'
  switch (record.activity.kind) {
    case 'settingUp':
      return review ? 'Checking out the PR…' : 'Running worktree setup…'
    case 'startingAgent':
      return 'Starting agent…'
    case 'agentStarted':
      if (record.status !== 'running') break
      return review ? 'Reading the diff…' : 'Reading repo…'
    case 'setupFailed':
      return 'Setup failed'
    case 'reviewReady':
      return 'Review ready'
    case 'reviewSubmitted':
      return 'Review submitted'
    case 'none':
      break
  }
  switch (record.status) {
    case 'provisioningFailed':
      return 'Setup failed'
    case 'provisioning':
      return record.kind === 'review'
        ? 'Checking out the PR…'
        : 'Running worktree setup…'
    case 'running':
      return 'Working'
    case 'ready':
      return 'Ready'
    default:
      return 'Idle'
  }
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

/**
 * Whether a workspace needs you: nothing is working in it, so it needs prompting or closing, or an
 * agent waits on an approval. `threadStatus` is its live thread's, which can be ahead of the
 * workspace's lifecycle.
 */
export function needsAttention(
  workspace: {
    lifecycle: WorkspaceLifecycle
    pendingApprovals: number
  },
  threadStatus: ThreadStatus | undefined,
): boolean {
  const working =
    (threadStatus !== undefined && isAgentWorking(threadStatus)) ||
    workspace.lifecycle === 'provisioning' ||
    workspace.lifecycle === 'running'
  return !working || workspace.pendingApprovals > 0
}
