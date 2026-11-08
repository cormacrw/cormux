import type { Snapshot } from '$lib/ipc/bindings'
import {
  approvalNeededNotificationBody,
  reviewFinishedText,
  reviewFinishedToast,
  type ToastPayload,
} from './toast-payload'
import type { ToastTarget } from './toast-target'

function openFindingsByWorkspace(snapshot: Snapshot): Map<string, number> {
  const map = new Map<string, number>()
  for (const finding of snapshot.persisted.findings) {
    if (finding.status !== 'open') continue
    map.set(finding.workspaceId, (map.get(finding.workspaceId) ?? 0) + 1)
  }
  return map
}

function prNumberForWorkspace(snapshot: Snapshot, workspaceId: string) {
  return (
    snapshot.persisted.workspaces.find((row) => row.id === workspaceId)
      ?.prNumber ?? null
  )
}

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
  toast: (payload: ToastPayload) => void
  notify: (body: string, target?: ToastTarget) => Promise<void>
  /** Whether the window has focus; notifications are for when it hasn't. */
  focused: () => boolean
}

/**
 * Compares each snapshot with the last: an approval newly waiting, or a review whose findings
 * just came in. The first snapshot is what was already there at launch, so it only sets the
 * baseline.
 */
export function createSupervisor(effects: SupervisorEffects) {
  let primed = false
  let lastPending = 0
  let lastOpenFindings = new Map<string, number>()

  return async function supervise(snapshot: Snapshot) {
    const pending = snapshot.pendingApprovals
    const openFindings = openFindingsByWorkspace(snapshot)
    const previousPending = lastPending
    const previousFindings = lastOpenFindings
    lastPending = pending
    lastOpenFindings = openFindings
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

    for (const [workspaceId, count] of openFindings) {
      if ((previousFindings.get(workspaceId) ?? 0) > 0) continue
      const prNumber = prNumberForWorkspace(snapshot, workspaceId)
      effects.toast(reviewFinishedToast(prNumber, count, workspaceId))
      if (!effects.focused() && snapshot.settings.notifyReviewFinished) {
        await effects.notify(reviewFinishedText(prNumber, count), {
          workspaceId,
          findings: true,
        })
      }
    }
  }
}
