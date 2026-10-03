import { app } from '$lib/state/app.svelte'
import { workspaceUi } from '$lib/state/workspace-ui.svelte'
import type { ToastPayload } from './toast-payload'

export type ToastTarget = Pick<
  ToastPayload,
  'workspaceId' | 'threadId' | 'findings' | 'scratchId'
>

/** Whether a toast or notification has somewhere to take you. */
export function hasToastTarget(target: ToastTarget): boolean {
  return Boolean(target.workspaceId || target.scratchId)
}

/** Opens what a toast or notification is about: a workspace's thread or Findings, or a scratch. */
export function openToastTarget(target: ToastTarget) {
  const { workspaceId, threadId, findings, scratchId } = target
  if (workspaceId) {
    app.openWorkspace(workspaceId, threadId)
    if (findings) {
      workspaceUi.findingsFocusPending = true
      workspaceUi.openTab('findings')
    } else if (threadId) {
      workspaceUi.openTab('thread')
    }
    return
  }
  if (scratchId) app.openScratch(scratchId)
}
