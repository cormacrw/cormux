import { commands } from '$lib/ipc'
import { settings } from '$lib/state/settings.svelte'
import { threads } from '$lib/state/threads.svelte'

export async function joinWorkspaceThread(workspaceId: string) {
  const count = threads.forWorkspace(workspaceId).length
  await commands.joinWorkspaceThread({
    workspaceId,
    title: `Agent ${count + 1}`,
    engine: settings.defaultEngine,
  })
}
