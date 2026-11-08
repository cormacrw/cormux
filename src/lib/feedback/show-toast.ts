import type { Component } from 'svelte'
import { toast } from 'svelte-sonner'
import { hasToastTarget, openToastTarget } from './toast-target'
import type { ToastPart, ToastPayload, ToastTone } from './toast-payload'
import { formatToastPayload } from './toast-payload'

/** What draws one toast; the toasts region hands it over as it mounts. */
export type ToastView = Component<{
  parts: ToastPart[]
  tone?: ToastTone
  onActivate?: () => void
}>

const TOAST_DURATION_MS = 3600

let view: ToastView | null = null
// Toasts raised before the region mounts, shown once it has.
let early: ToastPayload[] = []

export function setToastView(component: ToastView) {
  view = component
  const waiting = early
  early = []
  for (const payload of waiting) showToast(payload)
}

export function showToast(payload: ToastPayload) {
  if (!view) {
    early.push(payload)
    return
  }
  toast.custom(view, {
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
