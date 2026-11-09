import {
  WORKING_THREAD_STATUSES,
  type Snapshot,
  type ThreadStatus,
  type WorkspaceLifecycle,
} from '$lib/ipc/bindings'

/**
 * The core's `workspaces::needs_attention`, which the harness has to answer as its workspaces
 * move; `attention.test.ts` holds it to the core's cases.
 */
export function needsAttention(
  lifecycle: WorkspaceLifecycle,
  own: ThreadStatus | null,
  pending: number,
): boolean {
  const working =
    (own !== null &&
      (WORKING_THREAD_STATUSES as readonly ThreadStatus[]).includes(own)) ||
    lifecycle === 'provisioning' ||
    lifecycle === 'running'
  return !working || pending > 0
}

/** The snapshot's `workspacePendingApprovals`, from its approvals. */
export function workspacePendingApprovals(
  snapshot: Snapshot,
): Snapshot['workspacePendingApprovals'] {
  const { threads, approvals } = snapshot.persisted
  const counts = new Map<string, number>()
  for (const approval of approvals) {
    if (approval.status !== 'pending') continue
    const thread = threads.find((row) => row.id === approval.threadId)
    if (thread) {
      counts.set(thread.workspaceId, (counts.get(thread.workspaceId) ?? 0) + 1)
    }
  }
  return [...counts].map(([workspaceId, count]) => ({ workspaceId, count }))
}

/** The snapshot's `workspacesNeedingAttention`, from the rest of it. */
export function workspacesNeedingAttention(snapshot: Snapshot): string[] {
  const { threads } = snapshot.persisted
  return snapshot.workspaces
    .filter((workspace) => {
      const ownId = snapshot.workspaceOwnThreads.find(
        (row) => row.workspaceId === workspace.id,
      )?.threadId
      const own = threads.find((row) => row.id === ownId)?.status ?? null
      const pending = snapshot.workspacePendingApprovals.find(
        (row) => row.workspaceId === workspace.id,
      )
      return needsAttention(workspace.status, own, pending?.count ?? 0)
    })
    .map((workspace) => workspace.id)
}
