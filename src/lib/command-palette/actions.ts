import { app } from '$lib/state/app.svelte'
import { settings } from '$lib/state/settings.svelte'
import { workspaceUi } from '$lib/state/workspace-ui.svelte'
import { workspaceRecords } from '$lib/state/workspace-records.svelte'

/** Hooks for COR-15 / COR-17 — palette closes before these run. */
export function runWorkspaceApp(
  workspaceId: string,
  action: 'run' | 'restart' | 'stop',
) {
  window.dispatchEvent(
    new CustomEvent('cormux:workspace-app', {
      detail: { workspaceId, action },
    }),
  )
  if (action === 'run') {
    workspaceRecords.setAppStatus(workspaceId, 'starting')
  }
  if (action === 'stop') {
    workspaceRecords.setAppStatus(workspaceId, 'stopped', null)
  }
}

export function pullWorkspace(workspaceId: string) {
  window.dispatchEvent(
    new CustomEvent('cormux:workspace-pull', { detail: { workspaceId } }),
  )
}

export function rebaseWorkspace(workspaceId: string) {
  window.dispatchEvent(
    new CustomEvent('cormux:workspace-rebase', { detail: { workspaceId } }),
  )
}

export function requestNewThread(workspaceId: string) {
  window.dispatchEvent(
    new CustomEvent('cormux:new-thread', { detail: { workspaceId } }),
  )
}

export function openSettingsSection(section: string) {
  settings.focusSection = section
  app.openSettings()
}

export function openWorkspaceFindings(workspaceId: string) {
  app.openWorkspace(workspaceId)
  workspaceUi.openTab('findings')
}

export function openWorkspaceThread(workspaceId: string, threadId: string) {
  app.openWorkspace(workspaceId, threadId)
  workspaceUi.openTab('thread')
}

export function toggleReduceMotion() {
  settings.toggleReduceMotion()
}
