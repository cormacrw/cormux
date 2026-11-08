import { app } from '$lib/state/app.svelte'
import { workspaceUi } from '$lib/state/workspace-ui.svelte'

export function openWorkspaceThread(workspaceId: string, threadId: string) {
  app.openWorkspace(workspaceId, threadId)
  workspaceUi.openTab('thread')
}

/** Opens the workspace on its Findings tab, with focus on the findings. */
export function openWorkspaceFindings(workspaceId: string, threadId?: string) {
  app.openWorkspace(workspaceId, threadId)
  workspaceUi.findingsFocusPending = true
  workspaceUi.openTab('findings')
}
