import { events, type CoreError } from '$lib/ipc'
import { coreErrorText } from '$lib/core/core-error'
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
    parts: payload.parts,
  })
}

/** A bad toast for a failure: a core error, something thrown, or a message; `fallback` when it says nothing. */
export function toastCoreError(
  error: unknown,
  fallback = 'Something went wrong',
) {
  const message =
    error instanceof Error
      ? error.message
      : typeof error === 'string'
        ? error
        : isCoreError(error)
          ? coreErrorText(error, fallback)
          : fallback
  showToast(coreErrorToast(message))
}

function isCoreError(error: unknown): error is CoreError {
  return typeof error === 'object' && error !== null && 'kind' in error
}

export function toastEnvironmentReloaded() {
  showToast(environmentReloadToast())
}
