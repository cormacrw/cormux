import type { ToastPart, ToastTone } from '$lib/ipc/bindings'

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
