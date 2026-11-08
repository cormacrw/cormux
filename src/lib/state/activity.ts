import type {
  ThreadStatus,
  WorkspaceLifecycle,
  WorkspaceRecord,
} from '$lib/ipc/bindings'

/** What a thread is doing, in words; `setup` is its workspace's, for a thread still being set up. */
export function threadActivity(
  status: ThreadStatus,
  setup?: { activity: string; provStep: number },
): string {
  switch (status) {
    case 'running':
      return 'Working'
    case 'paused':
      return 'Paused'
    case 'provisioning':
      return setup?.provStep === 2 || setup?.activity.includes('Starting agent')
        ? 'Starting agent…'
        : 'Setting up the worktree'
    default:
      return 'Idle'
  }
}

/** What a workspace is doing: the core's own words while it has some, else its lifecycle's. */
export function workspaceActivity(
  record: Pick<WorkspaceRecord, 'status' | 'kind' | 'activity'>,
): string {
  if (record.activity.trim()) return record.activity
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

/**
 * Whether a workspace needs you: nothing is working in it, so it needs prompting or closing, or an
 * agent waits on an approval. `threadStatus` is its live thread's, which can be ahead of the
 * workspace's lifecycle.
 */
export function needsAttention(
  workspace: {
    lifecycle: WorkspaceLifecycle
    paused: boolean
    pendingApprovals: number
  },
  threadStatus: ThreadStatus | undefined,
): boolean {
  const working =
    (threadStatus !== undefined && isAgentWorking(threadStatus)) ||
    workspace.lifecycle === 'provisioning' ||
    (workspace.lifecycle === 'running' && !workspace.paused)
  return !working || workspace.pendingApprovals > 0
}
