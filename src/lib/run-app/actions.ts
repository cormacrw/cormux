import { commands } from '$lib/ipc'
import type { WorkspaceAppControlAction } from '$lib/ipc/bindings'
import { toastCoreError } from '$lib/feedback/wire-feedback'
import { workspaces } from '$lib/state/workspaces.svelte'
import { appActionAllowed, appControlsFor, type AppControls } from './controls'

export function appControls(workspaceId: string): AppControls {
  const workspace = workspaces.getById(workspaceId)
  return appControlsFor({
    appStatus: workspaces.runtime(workspaceId).appStatus,
    runBlock: workspace ? workspace.runBlock : 'No such workspace.',
    runCommand: workspace?.run?.command ?? '',
  })
}

const logClearListeners = new Set<(workspaceId: string) => void>()

/** Calls `listener` with the workspace whenever its log is cleared; returns the unsubscribe. */
export function onLogCleared(listener: (workspaceId: string) => void) {
  logClearListeners.add(listener)
  return () => {
    logClearListeners.delete(listener)
  }
}

/** Runs, restarts, stops or clears a workspace's app; does nothing when the action doesn't apply now. */
export async function controlApp(
  workspaceId: string,
  action: WorkspaceAppControlAction,
) {
  if (!appActionAllowed(appControls(workspaceId), action)) return
  if (action === 'clear') {
    for (const listener of logClearListeners) listener(workspaceId)
  }
  const result = await commands.controlWorkspaceApp({ workspaceId, action })
  if (result.status === 'error') toastCoreError(result.error)
}
