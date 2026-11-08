import { app } from '$lib/state/app.svelte'
import { openWorkspaceFindings, openWorkspaceThread } from '$lib/workspace/open'
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
    if (findings) openWorkspaceFindings(workspaceId, threadId)
    else if (threadId) openWorkspaceThread(workspaceId, threadId)
    else app.openWorkspace(workspaceId)
    return
  }
  if (scratchId) app.openScratch(scratchId)
}
