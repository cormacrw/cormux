import type { Snapshot } from '$lib/ipc/bindings'
import { approvalNeededNotificationBody } from './toast-payload'
import type { ToastTarget } from './toast-target'

/** The thread behind the newest pending approval, to open from its notification. */
function newestApprovalTarget(snapshot: Snapshot): ToastTarget | undefined {
  const approval = snapshot.persisted.approvals
    .filter((row) => row.status === 'pending')
    .at(-1)
  const thread = snapshot.persisted.threads.find(
    (row) => row.id === approval?.threadId,
  )
  if (!thread) return undefined
  const scratch = snapshot.persisted.scratches.find(
    (row) => row.threadId === thread.id,
  )
  if (scratch) return { scratchId: scratch.id }
  return { workspaceId: thread.workspaceId, threadId: thread.id }
}

type SupervisorEffects = {
  notify: (body: string, target?: ToastTarget) => Promise<void>
  /** Whether the window has focus; notifications are for when it hasn't. */
  focused: () => boolean
}

/**
 * Compares each snapshot with the last for an approval newly waiting. The first snapshot is what
 * was already there at launch, so it only sets the baseline.
 */
export function createSupervisor(effects: SupervisorEffects) {
  let primed = false
  let lastPending = 0

  return async function supervise(snapshot: Snapshot) {
    const pending = snapshot.pendingApprovals
    const previousPending = lastPending
    lastPending = pending
    if (!primed) {
      primed = true
      return
    }

    if (
      pending > previousPending &&
      !effects.focused() &&
      snapshot.settings.notifyApprovals
    ) {
      await effects.notify(
        approvalNeededNotificationBody(pending),
        newestApprovalTarget(snapshot),
      )
    }
  }
}
