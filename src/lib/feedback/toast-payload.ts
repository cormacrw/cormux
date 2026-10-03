/** Matches product/features/03-toasts-and-feedback.md tones. */
export type ToastTone = 'ok' | 'bad' | 'default'

export type ToastPart =
  { type: 'text'; value: string } | { type: 'code'; value: string }

export type ToastPayload = {
  tone: ToastTone
  parts: ToastPart[]
  /** When set, clicking the toast opens this workspace (COR-81). */
  workspaceId?: string
  /** With `workspaceId`, opens this thread's tab rather than the workspace's first. */
  threadId?: string
  /** With `workspaceId`, opens the Findings tab. */
  findings?: boolean
  /** When set, clicking the toast opens this scratch. */
  scratchId?: string
}

export function pendingApprovalCount(
  approvals: { status: string }[],
  livePending = 0,
): number {
  const stored = approvals.filter((row) => row.status === 'pending').length
  return Math.max(stored, livePending)
}

export function dockBadgeCount(pendingApprovals: number): number | undefined {
  if (pendingApprovals <= 0) return undefined
  return pendingApprovals
}

export function formatToastPayload(payload: ToastPayload): string {
  return payload.parts
    .map((part) => (part.type === 'code' ? part.value : part.value))
    .join('')
}

export function workspaceAppToast(
  name: string,
  action: 'run' | 'restart' | 'stop',
  port: number | null,
  workspaceId: string,
): ToastPayload {
  if (action === 'stop') {
    return {
      tone: 'default',
      workspaceId,
      parts: [
        { type: 'text', value: 'Stopped the app in ' },
        { type: 'code', value: name },
      ],
    }
  }
  if (action === 'restart') {
    return {
      tone: 'default',
      workspaceId,
      parts: [
        { type: 'text', value: 'Restarting the app in ' },
        { type: 'code', value: name },
      ],
    }
  }
  const portText = port ?? 5173
  return {
    tone: 'ok',
    workspaceId,
    parts: [
      { type: 'code', value: name },
      { type: 'text', value: ' is running on localhost:' },
      { type: 'code', value: String(portText) },
    ],
  }
}

export function pullToast(
  name: string,
  commits: number,
  base: string,
  workspaceId: string,
): ToastPayload {
  return {
    tone: 'ok',
    workspaceId,
    parts: [
      { type: 'text', value: 'Pulled ' },
      { type: 'code', value: String(commits) },
      { type: 'text', value: ` commit${commits === 1 ? '' : 's'} from ` },
      { type: 'code', value: base },
    ],
  }
}

export function newThreadToast(
  name: string,
  workspaceId: string,
): ToastPayload {
  return {
    tone: 'ok',
    workspaceId,
    parts: [
      { type: 'text', value: 'Started a new thread in ' },
      { type: 'code', value: name },
    ],
  }
}

export function environmentReloadToast(): ToastPayload {
  return {
    tone: 'default',
    parts: [{ type: 'text', value: 'Reloaded shell environment' }],
  }
}

export function coreErrorToast(message: string): ToastPayload {
  return {
    tone: 'bad',
    parts: [{ type: 'text', value: message }],
  }
}

export function reviewFinishedToast(
  prNumber: number | null,
  findings: number,
  workspaceId: string,
): ToastPayload {
  const head =
    prNumber != null
      ? `Review of #${prNumber} finished · `
      : 'Review finished · '
  return {
    tone: 'ok',
    workspaceId,
    findings: true,
    parts: [
      { type: 'text', value: head },
      { type: 'code', value: String(findings) },
      { type: 'text', value: ` finding${findings === 1 ? '' : 's'}` },
    ],
  }
}

export function approvalNeededNotificationBody(pending: number): string {
  if (pending === 1) return 'An agent is waiting for your approval.'
  return `${pending} agents are waiting for your approval.`
}
