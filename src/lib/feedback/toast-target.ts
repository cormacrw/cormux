import {
  openWorkspace,
  openWorkspaceFindings,
  openWorkspaceThread,
} from '$lib/nav/open'
import type { ToastPayload } from './toast-payload'

export type ToastTarget = Pick<
  ToastPayload,
  'workspaceId' | 'threadId' | 'findings'
>

export function hasToastTarget(target: ToastTarget): boolean {
  return Boolean(target.workspaceId)
}

export function openToastTarget(target: ToastTarget) {
  const { workspaceId, threadId, findings } = target
  if (!workspaceId) return
  if (findings) openWorkspaceFindings(workspaceId, threadId)
  else if (threadId) openWorkspaceThread(workspaceId, threadId)
  else openWorkspace(workspaceId)
}
