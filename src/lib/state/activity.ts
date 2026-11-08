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

/** Agents working or starting, which hold the workspace's branch where it is. */
export function runningAgentCount(
  threads: { status: ThreadStatus; paused: boolean }[],
): number {
  return threads.filter(
    (thread) =>
      (thread.status === 'running' && !thread.paused) ||
      thread.status === 'provisioning',
  ).length
}
