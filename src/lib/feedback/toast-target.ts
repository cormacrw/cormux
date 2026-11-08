import { navigate } from '$lib/core/nav.svelte'
import {
  openWorkspace,
  openWorkspaceFindings,
  openWorkspaceThread,
} from '$lib/nav/open'
import type { ToastPayload } from './toast-payload'

export type ToastTarget = Pick<
  ToastPayload,
  'workspaceId' | 'threadId' | 'findings' | 'scratchId'
>

export function hasToastTarget(target: ToastTarget): boolean {
  return Boolean(target.workspaceId || target.scratchId)
}

export function openToastTarget(target: ToastTarget) {
  const { workspaceId, threadId, findings, scratchId } = target
  if (workspaceId) {
    if (findings) openWorkspaceFindings(workspaceId, threadId)
    else if (threadId) openWorkspaceThread(workspaceId, threadId)
    else openWorkspace(workspaceId)
    return
  }
  if (scratchId) navigate({ view: 'scratch', scratchId })
}
