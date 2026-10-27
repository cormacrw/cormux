import { events, type CoreError } from '$lib/ipc'
import { coreErrorText } from './core-error'
import type { ToastRaised } from '$lib/ipc/bindings'
import { workspaces } from '$lib/state/workspaces.svelte'
import {
  coreErrorToast,
  environmentReloadToast,
  newThreadToast,
} from './toast-payload'
import { showToast } from './show-toast'

function workspaceName(workspaceId: string) {
  return workspaces.getById(workspaceId)?.name ?? workspaceId
}

export async function bindFeedbackEvents() {
  const cleanups: (() => void)[] = []

  cleanups.push(
    await events.toastRaised.listen((event) => {
      showCoreToast(event.payload)
    }),
  )

  const onNewThread = (event: Event) => {
    const detail = (event as CustomEvent).detail as { workspaceId: string }
    showToast(
      newThreadToast(workspaceName(detail.workspaceId), detail.workspaceId),
    )
    void import('$lib/workspace/join-thread').then(({ joinWorkspaceThread }) =>
      joinWorkspaceThread(detail.workspaceId),
    )
  }

  window.addEventListener('cormux:new-thread', onNewThread)

  cleanups.push(() => {
    window.removeEventListener('cormux:new-thread', onNewThread)
  })

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
