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

export function formatToastPayload(payload: ToastPayload): string {
  return payload.parts.map((part) => part.value).join('')
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
