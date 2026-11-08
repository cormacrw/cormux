import type { ToastPart, ToastTone } from '$lib/ipc/bindings'
import { plural, pluralWord } from '$lib/format/plural'

export type { ToastPart, ToastTone }

/** Tones as product/features/03-toasts-and-feedback.md describes them. */
export type ToastPayload = {
  tone: ToastTone
  parts: ToastPart[]
  /** When set, clicking the toast opens this workspace. */
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

function reviewFinishedHead(prNumber: number | null) {
  return prNumber != null
    ? `Review of #${prNumber} finished`
    : 'Review finished'
}

/** `Review of #12 finished · 3 findings`, as a notification says it. */
export function reviewFinishedText(prNumber: number | null, findings: number) {
  return `${reviewFinishedHead(prNumber)} · ${plural(findings, 'finding')}`
}

/** The same, as a toast that opens the workspace's Findings, with the count set as code. */
export function reviewFinishedToast(
  prNumber: number | null,
  findings: number,
  workspaceId: string,
): ToastPayload {
  return {
    tone: 'ok',
    workspaceId,
    findings: true,
    parts: [
      { type: 'text', value: `${reviewFinishedHead(prNumber)} · ` },
      { type: 'code', value: String(findings) },
      { type: 'text', value: ` ${pluralWord(findings, 'finding')}` },
    ],
  }
}

export function approvalNeededNotificationBody(pending: number): string {
  if (pending === 1) return 'An agent is waiting for your approval.'
  return `${pending} agents are waiting for your approval.`
}
