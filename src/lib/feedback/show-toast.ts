import { toast } from 'svelte-sonner'
import { hasToastTarget, openToastTarget } from './toast-target'
import type { ToastPayload } from './toast-payload'
import { formatToastPayload } from './toast-payload'
import ToastLine from './ToastLine.svelte'

const TOAST_DURATION_MS = 3600

/** Raised before the toasts region mounts, it shows once the region has. */
export function showToast(payload: ToastPayload) {
  toast.custom(ToastLine, {
    componentProps: {
      parts: payload.parts,
      tone: payload.tone,
      onActivate: hasToastTarget(payload)
        ? () => openToastTarget(payload)
        : undefined,
    },
    duration: TOAST_DURATION_MS,
    dismissible: false,
    important: false,
    description: formatToastPayload(payload),
  })
}
