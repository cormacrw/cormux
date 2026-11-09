import type { WorkspaceLifecycle } from '$lib/ipc/bindings'

/**
 * Waiting: the agent is on hold until the user answers it, such as reviewing its plan.
 * Ready: nothing is happening and it's your move. It holds still, since a sidebar of finished
 * threads shouldn't keep the app animating.
 */
export type StatusDotVariant =
  'provisioning' | 'running' | 'waiting' | 'ready' | 'paused' | 'idle'

/**
 * A workspace's dot. One that needs you (`needsAttention`) says so first, flashing only while an
 * agent is on hold for an answer; an approval can land while it still reads as running. Otherwise
 * the dot follows its lifecycle.
 */
export function statusDotFor(
  workspace: { lifecycle: WorkspaceLifecycle; pendingApprovals: number },
  attention: boolean,
): StatusDotVariant {
  if (attention) return workspace.pendingApprovals > 0 ? 'waiting' : 'ready'
  switch (workspace.lifecycle) {
    case 'provisioning':
    case 'running':
    case 'paused':
      return workspace.lifecycle
    default:
      return 'idle'
  }
}
