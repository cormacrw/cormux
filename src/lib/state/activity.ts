import type {
  ThreadStatus,
  WorkspaceActivity,
  WorkspaceLifecycle,
  WorkspaceRecord,
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

/** What a workspace is doing: what the core says it's at, else its lifecycle, in words. */
export function workspaceActivity(
  record: Pick<WorkspaceRecord, 'status' | 'kind' | 'activity'>,
): string {
  const review = record.kind === 'review'
  switch (record.activity.kind) {
    case 'settingUp':
      return review ? 'Checking out the PR…' : 'Running worktree setup…'
    case 'startingAgent':
      return 'Starting agent…'
    case 'agentStarted':
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

/** An agent working or starting: what holds a workspace's branch where it is. */
export function isAgentWorking(status: ThreadStatus): boolean {
  return status === 'running' || status === 'provisioning'
}

export function runningAgentCount(threads: { status: ThreadStatus }[]): number {
  return threads.filter((thread) => isAgentWorking(thread.status)).length
}
