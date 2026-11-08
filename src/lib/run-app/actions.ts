import { commands } from '$lib/ipc'
import type { WorkspaceAppControlAction } from '$lib/ipc/bindings'
import { toastCoreError } from '$lib/feedback/wire-feedback'
import { repos } from '$lib/state/repos.svelte'
import { workspaces } from '$lib/state/workspaces.svelte'
import { workspaceRunCommand } from './run-command'
import { appActionAllowed, appControlsFor, type AppControls } from './controls'

export function appControls(workspaceId: string): AppControls {
  const workspace = workspaces.getById(workspaceId)
  return appControlsFor({
    appStatus: workspaces.runtime(workspaceId).appStatus,
    lifecycle: workspace?.lifecycle,
    runCommand: workspaceRunCommand(
      workspace?.runCommand,
      repos.runCommand(workspace?.repoId),
    ),
  })
}

/** Runs, restarts, stops or clears a workspace's app; does nothing when the action doesn't apply now. */
export async function controlApp(
  workspaceId: string,
  action: WorkspaceAppControlAction,
) {
  if (!appActionAllowed(appControls(workspaceId), action)) return
  if (action === 'run') workspaces.setAppStatus(workspaceId, 'starting')
  if (action === 'clear') workspaces.clearLog(workspaceId)
  const result = await commands.controlWorkspaceApp({ workspaceId, action })
  if (result.status === 'error') {
    toastCoreError(result.error)
    if (action === 'run') {
      workspaces.setAppStatus(workspaceId, 'stopped', null)
    }
  }
}
