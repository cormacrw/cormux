import type { Snapshot } from '$lib/ipc'
import { readBooleanSetting } from '$lib/new-workspace/settings-defaults'
import { syncDockBadge } from './dock-badge'
import { notifyHarness } from './os-notifications'
import {
  approvalNeededNotificationBody,
  pendingApprovalCount,
  reviewFinishedToast,
} from './toast-payload'
import { showToast } from './show-toast'
import type { ToastTarget } from './toast-target'

type ReviewSignature = Map<string, number>

let lastPending = 0
let lastOpenFindings: ReviewSignature = new Map()
let windowFocused = true

export function setHarnessWindowFocused(focused: boolean) {
  windowFocused = focused
}

function openFindingsByWorkspace(snapshot: Snapshot): ReviewSignature {
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

export async function onSnapshotSupervision(snapshot: Snapshot) {
  const pending = pendingApprovalCount(
    snapshot.persisted.approvals,
    snapshot.pendingLiveApprovals,
  )
  await syncDockBadge(pending)

  const notifyApprovals = readBooleanSetting(
    snapshot.persisted.settings,
    'notifyApprovals',
    true,
  )
  if (pending > lastPending && !windowFocused && notifyApprovals) {
    await notifyHarness(
      'Cormux',
      approvalNeededNotificationBody(pending),
      newestApprovalTarget(snapshot),
    )
  }
  lastPending = pending

  const openFindings = openFindingsByWorkspace(snapshot)
  for (const [workspaceId, count] of openFindings) {
    const previous = lastOpenFindings.get(workspaceId) ?? 0
    if (count > previous && previous === 0) {
      showToast(
        reviewFinishedToast(
          prNumberForWorkspace(snapshot, workspaceId),
          count,
          workspaceId,
        ),
      )
      const notifyReviewFinished = readBooleanSetting(
        snapshot.persisted.settings,
        'notifyReviewFinished',
        true,
      )
      if (!windowFocused && notifyReviewFinished) {
        const label =
          prNumberForWorkspace(snapshot, workspaceId) != null
            ? `Review of #${prNumberForWorkspace(snapshot, workspaceId)} finished · ${count} findings`
            : `Review finished · ${count} findings`
        await notifyHarness('Cormux', label, { workspaceId, findings: true })
      }
    }
  }
  lastOpenFindings = openFindings
}

export function resetSupervisionStateForTests() {
  lastPending = 0
  lastOpenFindings = new Map()
  windowFocused = true
}
