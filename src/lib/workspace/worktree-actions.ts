import { commands } from '$lib/ipc'
import { toastCoreError } from '$lib/feedback/wire-feedback'

export async function openWorkspaceTerminal(workspaceId: string) {
  const result = await commands.openWorkspaceTerminal(workspaceId)
  if (result.status === 'error') toastCoreError(result.error)
}
