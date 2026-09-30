import { commands } from '$lib/ipc'
import { app } from '$lib/state/app.svelte'
import { threads } from '$lib/state/threads.svelte'
import { coreErrorToast } from '$lib/feedback/toast-payload'
import { showToast } from '$lib/feedback/show-toast'

/** The workspace's first thread (Lead, Planner or Reviewer) owns the worktree and stays open. */
export function canCloseThread(workspaceId: string, threadId: string): boolean {
  return threads.forWorkspace(workspaceId)[0]?.id !== threadId
}

/** Stop a thread's agent and drop its tab; its history stays in the database. */
export async function closeThreadTab(workspaceId: string, threadId: string) {
  if (!canCloseThread(workspaceId, threadId)) return
  const siblings = threads.forWorkspace(workspaceId)
  const index = siblings.findIndex((thread) => thread.id === threadId)
  const previous = threads.items
  if (app.threadId === threadId) {
    app.threadId = (siblings[index - 1] ?? siblings[index + 1])?.id ?? null
  }
  threads.hydrate(previous.filter((thread) => thread.id !== threadId))
  const result = await commands.closeWorkspaceThread(threadId)
  if (result.status === 'error') {
    threads.hydrate(previous)
    const message =
      typeof result.error.message === 'string'
        ? result.error.message
        : 'Could not close the thread'
    showToast(coreErrorToast(message))
  }
}
