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
  const workspace = snapshot.workspaces.find((row) => row.id === workspaceId)
  if (!workspace) return null
  const pr = snapshot.persisted.pullRequests.find(
    (row) => row.repoId === workspace.repoId,
  )
  return pr?.number ?? null
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
    await notifyHarness('Cormux', approvalNeededNotificationBody(pending))
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
        await notifyHarness('Cormux', label)
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
