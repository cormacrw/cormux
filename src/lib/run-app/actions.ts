import { commands } from '$lib/ipc'
import type { WorkspaceAppControlAction } from '$lib/ipc/bindings'
import { toastCoreError } from '$lib/feedback/wire-feedback'
import { repos } from '$lib/state/repos.svelte'
import { workspaceRecords } from '$lib/state/workspace-records.svelte'
import { workspaces } from '$lib/state/workspaces.svelte'
import { workspaceRunCommand } from './run-command'
import { appActionAllowed, appControlsFor, type AppControls } from './controls'

export function appControls(workspaceId: string): AppControls {
  const record = workspaceRecords.getRecord(workspaceId)
  return appControlsFor({
    appStatus: workspaceRecords.runtime(workspaceId).appStatus,
    lifecycle: workspaces.getById(workspaceId)?.lifecycle,
    runCommand: workspaceRunCommand(
      workspaces.getById(workspaceId)?.runCommand,
      repos.runCommand(record?.repoId),
    ),
  })
}

/** Runs, restarts, stops or clears a workspace's app; does nothing when the action doesn't apply now. */
export async function controlApp(
  workspaceId: string,
  action: WorkspaceAppControlAction,
) {
  if (!appActionAllowed(appControls(workspaceId), action)) return
  if (action === 'run') workspaceRecords.setAppStatus(workspaceId, 'starting')
  if (action === 'clear') workspaceRecords.clearLog(workspaceId)
  const result = await commands.controlWorkspaceApp({ workspaceId, action })
  if (result.status === 'error') {
    toastCoreError(result.error)
    if (action === 'run') {
      workspaceRecords.setAppStatus(workspaceId, 'stopped', null)
    }
  }
}
