import { events } from '$lib/ipc'
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
        : 'Something went wrong'
  showToast(coreErrorToast(message))
}

export function toastEnvironmentReloaded() {
  showToast(environmentReloadToast())
}
