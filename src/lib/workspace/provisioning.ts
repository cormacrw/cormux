import type { WorkspaceLifecycle, WorkspaceRecord } from '$lib/ipc/bindings'

export function workspaceActivityFromRecord(
  lifecycle: WorkspaceLifecycle,
  record: WorkspaceRecord | undefined,
  kind: 'review' | null,
): string {
  if (record?.activity?.trim()) {
    return record.activity
  }
  if (lifecycle === 'provisioningFailed') return 'Setup failed'
  if (lifecycle === 'provisioning' || lifecycle === 'creating') {
    return kind === 'review'
      ? 'Checking out the PR…'
      : 'Running worktree setup…'
  }
  if (lifecycle === 'running') return 'Working'
  if (lifecycle === 'waiting') return 'Waiting for approval'
  if (lifecycle === 'ready') return 'Ready'
  return 'Idle'
}

export function threadActivityForProvisioning(
  status: string,
  workspaceActivity: string,
  provStep: number,
): string {
  if (status !== 'provisioning') {
    if (status === 'running') return 'Working'
    if (status === 'waiting') return 'Waiting for approval'
    if (status === 'paused') return 'Paused'
    return 'Idle'
  }
  if (workspaceActivity.includes('Joining')) return 'Joining the worktree'
  if (provStep === 2 || workspaceActivity.includes('Starting agent')) {
    return 'Starting agent…'
  }
  return 'Setting up the worktree'
}

export function isWorkspaceProvisioning(
  lifecycle: WorkspaceLifecycle,
): boolean {
  return lifecycle === 'creating' || lifecycle === 'provisioning'
}
