import { commands } from '$lib/ipc'
import { app } from '$lib/state/app.svelte'
import { settings } from '$lib/state/settings.svelte'
import { threads } from '$lib/state/threads.svelte'
import { workspaceUi } from '$lib/state/workspace-ui.svelte'
import { coreErrorToast } from '$lib/feedback/toast-payload'
import { showToast } from '$lib/feedback/show-toast'

export async function joinWorkspaceThread(workspaceId: string) {
  const count = threads.forWorkspace(workspaceId).length
  const result = await commands.joinWorkspaceThread({
    workspaceId,
    title: `Agent ${count + 1}`,
    engine: settings.defaultEngine,
  })
  if (result.status !== 'ok') {
    const message =
      typeof result.error === 'object' &&
      result.error &&
      'message' in result.error
        ? String(result.error.message)
        : 'Could not start thread'
    showToast(coreErrorToast(message))
    return
  }
  app.openWorkspace(workspaceId, result.data.threadId)
  workspaceUi.openTab('thread')
}
