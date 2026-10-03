import { toast } from 'svelte-sonner'
import ToastLine from '$lib/components/feedback/ToastLine.svelte'
import { hasToastTarget, openToastTarget } from './toast-target'
import type { ToastPayload } from './toast-payload'
import { formatToastPayload } from './toast-payload'

const TOAST_DURATION_MS = 3600

/** Show one Harness toast (spec §03). */
export function showToast(payload: ToastPayload) {
  const label = formatToastPayload(payload)

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
    description: label,
  })
}
