import type { WorkspaceLifecycle } from '$lib/ipc/bindings'

/**
 * Waiting: the agent is on hold until the user answers it, such as reviewing its plan.
 * Ready: nothing is happening and it's your move. It holds still, since a sidebar of finished
 * threads shouldn't keep the app animating.
 */
export type StatusDotVariant =
  'provisioning' | 'running' | 'waiting' | 'ready' | 'paused' | 'idle'

type WorkspaceSidebarInput = {
  lifecycle: WorkspaceLifecycle
  paused: boolean
  activityText: string
}

export function statusDotVariantForWorkspace(
  workspace: WorkspaceSidebarInput,
): StatusDotVariant {
  if (
    workspace.lifecycle === 'creating' ||
    workspace.lifecycle === 'provisioning'
  ) {
    return 'provisioning'
  }
  if (workspace.lifecycle === 'running') {
    return workspace.paused ? 'paused' : 'running'
  }
  if (workspace.lifecycle === 'waiting') return 'waiting'
  return 'idle'
}

/** The dot for a thread that needs you: only an agent on hold for an answer is worth flashing for. */
export function statusDotVariantForAttention(
  pendingApprovals: number,
  lifecycle: WorkspaceLifecycle | undefined,
): StatusDotVariant {
  return pendingApprovals > 0 || lifecycle === 'waiting' ? 'waiting' : 'ready'
}

export function workspaceStatusWord(workspace: WorkspaceSidebarInput): string {
  if (
    workspace.lifecycle === 'creating' ||
    workspace.lifecycle === 'provisioning'
  ) {
    return 'Starting'
  }
  if (workspace.lifecycle === 'running') {
    return workspace.paused ? 'Paused' : 'Running'
  }
  return workspace.activityText
}
