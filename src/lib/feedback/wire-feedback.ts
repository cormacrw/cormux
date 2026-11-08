import { events, type CoreError } from '$lib/ipc'
import { coreErrorText } from './core-error'
import type { ToastRaised } from '$lib/ipc/bindings'
import { coreErrorToast, environmentReloadToast } from './toast-payload'
import { showToast } from './show-toast'

export function bindFeedbackEvents() {
  return events.toastRaised.listen((event) => showCoreToast(event.payload))
}

function showCoreToast(event: ToastRaised) {
  const payload = event.payload
  showToast({
    tone: payload.tone,
    workspaceId: payload.workspaceId ?? undefined,
    threadId: payload.threadId ?? undefined,
    parts: payload.parts.map((part) =>
      part.type === 'code'
        ? { type: 'code', value: part.value }
        : { type: 'text', value: part.value },
    ),
  })
}

export function toastCoreError(error: unknown) {
  const message =
    error instanceof Error
      ? error.message
      : typeof error === 'string'
        ? error
        : isCoreError(error)
          ? coreErrorText(error)
          : 'Something went wrong'
  showToast(coreErrorToast(message))
}

function isCoreError(error: unknown): error is CoreError {
  return typeof error === 'object' && error !== null && 'kind' in error
}

export function toastEnvironmentReloaded() {
  showToast(environmentReloadToast())
}
