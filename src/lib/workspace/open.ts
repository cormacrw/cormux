import { nav, navigate } from '$lib/core/nav.svelte'
import type { WorkspacePanel } from '$lib/core/route'
import { threads } from '$lib/state/threads.svelte'

/** Opens a workspace, on its live thread unless `threadId` names another. */
export function openWorkspace(
  workspaceId: string,
  at: {
    threadId?: string | null
    panel?: WorkspacePanel
    reveal?: { path: string; line: number | null }
  } = {},
) {
  navigate({
    view: 'workspace',
    workspaceId,
    threadId: at.threadId ?? threads.liveFor(workspaceId)?.id ?? null,
    panel: at.panel,
    reveal: at.reveal,
  })
}

/** Shows this thread's conversation, in its workspace. */
export function openWorkspaceThread(workspaceId: string, threadId: string) {
  openWorkspace(workspaceId, { threadId, panel: 'thread' })
}

/** Opens the workspace on its Findings tab, with focus on the findings. */
export function openWorkspaceFindings(workspaceId: string, threadId?: string) {
  openWorkspace(workspaceId, { threadId, panel: 'findings' })
}

/** Swaps the open workspace's conversation for another of its threads, staying on the panel shown. */
export function showThread(threadId: string) {
  const workspaceId = nav.workspaceId
  if (workspaceId) openWorkspace(workspaceId, { threadId })
}

/** Opens the open workspace's Changes at a file, and at a line of it when given. */
export function revealInChanges(path: string, line: number | null = null) {
  const workspaceId = nav.workspaceId
  if (!workspaceId) return
  openWorkspace(workspaceId, {
    threadId: nav.threadId,
    panel: 'changes',
    reveal: { path, line },
  })
}
