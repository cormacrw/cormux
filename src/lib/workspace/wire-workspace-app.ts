import { commands } from '$lib/ipc'
import { toastCoreError } from '$lib/feedback/wire-feedback'
import { workspaceRecords } from '$lib/state/workspace-records.svelte'
import { rememberHeaderFocusKey } from '$lib/workspace/header-focus'

export function bindWorkspaceAppControls() {
  const onWorkspaceApp = (event: Event) => {
    const detail = (event as CustomEvent).detail as {
      workspaceId: string
      action: 'run' | 'restart' | 'stop' | 'clear'
    }
    void controlWorkspaceApp(detail.workspaceId, detail.action)
  }

  window.addEventListener('cormux:workspace-app', onWorkspaceApp)
  return () =>
    window.removeEventListener('cormux:workspace-app', onWorkspaceApp)
}

async function controlWorkspaceApp(
  workspaceId: string,
  action: 'run' | 'restart' | 'stop' | 'clear',
) {
  if (action === 'run') {
    workspaceRecords.setAppStatus(workspaceId, 'starting')
    rememberHeaderFocusKey('stop')
  }
  if (action === 'stop') {
    rememberHeaderFocusKey('run')
  }
  if (action === 'clear') {
    workspaceRecords.clearLog(workspaceId)
  }

  const result = await commands.controlWorkspaceApp({ workspaceId, action })
  if (result.status === 'error') {
    toastCoreError(result.error)
    if (action === 'run') {
      workspaceRecords.setAppStatus(workspaceId, 'stopped', null)
    }
  }
}
