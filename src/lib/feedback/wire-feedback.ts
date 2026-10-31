import { events, type CoreError } from '$lib/ipc'
import { coreErrorText } from './core-error'
import type { ToastRaised } from '$lib/ipc/bindings'
import { coreErrorToast, environmentReloadToast } from './toast-payload'
import { showToast } from './show-toast'

export async function bindFeedbackEvents() {
  const cleanups: (() => void)[] = []

  cleanups.push(
    await events.toastRaised.listen((event) => {
      showCoreToast(event.payload)
    }),
  )

  return () => {
    for (const cleanup of cleanups) cleanup()
  }
}

function mapTone(tone: ToastRaised['payload']['tone']) {
  if (tone === 'ok') return 'ok' as const
  if (tone === 'bad') return 'bad' as const
  return 'default' as const
}

export function showCoreToast(event: ToastRaised) {
  const payload = event.payload
  showToast({
    tone: mapTone(payload.tone),
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
